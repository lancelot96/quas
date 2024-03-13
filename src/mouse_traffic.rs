use std::{path::PathBuf, process::Output};

use anyhow::Result;
use async_trait::async_trait;
use plotters::{
    backend::BitMapBackend,
    chart::ChartBuilder,
    drawing::IntoDrawingArea,
    element::Circle,
    series::PointSeries,
    style::{Color, BLACK, BLUE, RED, WHITE},
};
use tokio::process::Command as Process;

use crate::{error::Error, Command};

#[derive(Debug)]
pub struct MouseTraffic {
    file: String,
}

impl MouseTraffic {
    pub fn new(file: String) -> Self {
        Self { file }
    }

    async fn packets_from_file(file: &str) -> Result<String> {
        let Output {
            status,
            stdout,
            stderr,
        } = Process::new("tshark")
            .args([
                "-r",
                file,
                "-2",
                "-R",
                "usb",
                "-T",
                "fields",
                "-e",
                "usb.capdata",
            ])
            .output()
            .await?;
        if !status.success() {
            let stderr = String::from_utf8(stderr)?;
            return Err(Error::Process(stderr).into());
        }

        String::from_utf8(stdout).map_err(Into::into)
    }

    fn traffic_from_packets(packets: &str) -> Vec<(u8, i8, i8)> {
        packets
            .lines()
            .filter(|x| x.len() == 8)
            .flat_map(|x| {
                u8::from_str_radix(&x[..2], 16).ok().zip(
                    u8::from_str_radix(&x[2..4], 16)
                        .ok()
                        .zip(u8::from_str_radix(&x[4..6], 16).ok()),
                )
            })
            .map(|(c, (x, y))| (c, x as i8, y as i8))
            .collect()
    }

    fn steg_from_traffic(traffic: Vec<(u8, i8, i8)>) -> MouseTracesWithBoundary {
        let (mut unclick, mut left, mut right) = (Vec::new(), Vec::new(), Vec::new());
        let (mut x, mut y) = (0_i64, 0_i64);
        let (mut x_min, mut x_max, mut y_min, mut y_max) = (i64::MAX, i64::MIN, i64::MAX, i64::MIN);
        for (c, dx, dy) in traffic {
            x += i64::from(dx);
            y -= i64::from(dy);

            x_min = x_min.min(x);
            x_max = x_max.max(x);
            y_min = y_min.min(y);
            y_max = y_max.max(y);

            match c {
                0 => unclick.push((x, y)),
                1 => left.push((x, y)),
                2 => right.push((x, y)),
                _ => {
                    tracing::warn!("Unknown traffic ({}:{}:{})", c, dx, dy);
                    continue;
                }
            }
        }
        tracing::debug!(x_min, x_max, y_min, y_max);
        tracing::trace!(?unclick, ?left, ?right);

        MouseTracesWithBoundary {
            x_min,
            x_max,
            y_min,
            y_max,
            unclick,
            left,
            right,
        }
    }

    fn draw(file: &str, traces: MouseTracesWithBoundary) -> Result<()> {
        let png_path = PathBuf::from(file)
            .file_stem()
            .and_then(|x| x.to_str())
            .map(|x| format!("{}.png", x))
            .unwrap();
        let root = BitMapBackend::new(&png_path, (1920, 1080)).into_drawing_area();
        root.fill(&WHITE)?;

        let MouseTracesWithBoundary {
            x_min,
            x_max,
            y_min,
            y_max,
            unclick,
            left,
            right,
        } = traces;
        let mut chart = ChartBuilder::on(&root).build_cartesian_2d(x_min..x_max, y_min..y_max)?;
        for (points, color) in [unclick, left, right].into_iter().zip([BLACK, BLUE, RED]) {
            chart.draw_series(PointSeries::<_, _, Circle<_, _>, _>::new(
                points,
                4,
                color.mix(0.6).filled(),
            ))?;
        }

        root.present()?;
        tracing::info!("Mouse trace saved as ({:?}).", png_path);

        Ok(())
    }
}

#[derive(Debug)]
struct MouseTracesWithBoundary {
    x_min: i64,
    x_max: i64,
    y_min: i64,
    y_max: i64,
    unclick: Vec<(i64, i64)>,
    left: Vec<(i64, i64)>,
    right: Vec<(i64, i64)>,
}

#[async_trait]
impl Command for MouseTraffic {
    async fn execute(self: Box<Self>) -> Result<()> {
        let Self { file } = *self;

        let packets = Self::packets_from_file(&file).await?;
        let traffic = Self::traffic_from_packets(&packets);
        let traces = Self::steg_from_traffic(traffic);

        Self::draw(&file, traces)
    }
}

#[cfg(test)]
mod tests {
    use super::{MouseTracesWithBoundary, MouseTraffic};

    #[test]
    fn test_traffic_from_packets() {
        let packets = "683a3135370d0a\n4f4b41598a0b00004a0700000000000000000000b0b4bea6\n0100000000000000\n00ff0000\n0000ff00\n0100060000000000";
        let traffic = MouseTraffic::traffic_from_packets(&packets);
        assert_eq!(traffic, vec![(0, -1, 0), (0, 0, -1)]);
    }

    #[test]
    fn test_steg_from_traffic_with_unclick() {
        let traffic = vec![(0, -1, 0), (0, 0, -1)];
        let MouseTracesWithBoundary {
            x_min,
            x_max,
            y_min,
            y_max,
            unclick,
            left,
            right,
        } = MouseTraffic::steg_from_traffic(traffic);
        assert_eq!(x_min, -1);
        assert_eq!(x_max, -1);
        assert_eq!(y_min, 0);
        assert_eq!(y_max, 1);
        assert_eq!(unclick, vec![(-1, 0), (-1, 1)]);
        assert_eq!(left, vec![]);
        assert_eq!(right, vec![]);
    }

    #[test]
    fn test_steg_from_traffic_with_left() {
        let traffic = vec![(1, -1, 0), (1, 0, -1)];
        let MouseTracesWithBoundary {
            x_min,
            x_max,
            y_min,
            y_max,
            unclick,
            left,
            right,
        } = MouseTraffic::steg_from_traffic(traffic);
        assert_eq!(x_min, -1);
        assert_eq!(x_max, -1);
        assert_eq!(y_min, 0);
        assert_eq!(y_max, 1);
        assert_eq!(unclick, vec![]);
        assert_eq!(left, vec![(-1, 0), (-1, 1)]);
        assert_eq!(right, vec![]);
    }

    #[test]
    fn test_steg_from_traffic_with_right() {
        let traffic = vec![(2, -1, 0), (2, 0, -1)];
        let MouseTracesWithBoundary {
            x_min,
            x_max,
            y_min,
            y_max,
            unclick,
            left,
            right,
        } = MouseTraffic::steg_from_traffic(traffic);
        assert_eq!(x_min, -1);
        assert_eq!(x_max, -1);
        assert_eq!(y_min, 0);
        assert_eq!(y_max, 1);
        assert_eq!(unclick, vec![]);
        assert_eq!(left, vec![]);
        assert_eq!(right, vec![(-1, 0), (-1, 1)]);
    }

    #[test]
    fn test_steg_from_traffic_with_mixed() {
        let traffic = vec![(0, 1, -1), (1, -1, 0), (2, 0, -1)];
        let MouseTracesWithBoundary {
            x_min,
            x_max,
            y_min,
            y_max,
            unclick,
            left,
            right,
        } = MouseTraffic::steg_from_traffic(traffic);
        assert_eq!(x_min, 0);
        assert_eq!(x_max, 1);
        assert_eq!(y_min, 1);
        assert_eq!(y_max, 2);
        assert_eq!(unclick, vec![(1, 1)]);
        assert_eq!(left, vec![(0, 1)]);
        assert_eq!(right, vec![(0, 2)]);
    }
}
