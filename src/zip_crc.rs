use std::{
    cmp::Ordering,
    collections::{BTreeMap, HashMap},
    fs::File,
    sync::Arc,
};

use anyhow::Result;
use async_trait::async_trait;
use tokio::{sync::Mutex, task::JoinSet};
use tracing::instrument;
use zip::ZipArchive;

use crate::{error::Error, Command};

type SolutionMap = HashMap<u32, (String, Mutex<Vec<String>>)>;

#[derive(Debug)]
pub struct ZipCrc {
    file: String,
    size: u64,
    alphabet: String,
}

impl ZipCrc {
    pub fn new(file: String, size: u64, alphabet: String) -> Self {
        Self {
            file,
            size,
            alphabet,
        }
    }

    #[instrument(skip(ctx))]
    async fn brute(first: char, ctx: Arc<Context>) {
        let mut curr = first.to_string();
        let mut stack = vec![ctx.alphabet.chars()];

        while let Some(cs) = stack.last_mut() {
            match curr.as_bytes().len().cmp(&ctx.size) {
                Ordering::Greater => (),
                Ordering::Equal => {
                    let crc = crc32fast::hash(curr.as_bytes());
                    if let Some((_, pts)) = ctx.crc2pts.get(&crc) {
                        tracing::trace!(curr);
                        pts.lock().await.push(curr.clone());
                    }
                }
                Ordering::Less => {
                    if let Some(c) = cs.next() {
                        curr.push(c);
                        stack.push(ctx.alphabet.chars());
                        continue;
                    }
                }
            }

            curr.pop();
            stack.pop();
        }
    }

    fn init_buckets(mut zip: ZipArchive<File>, size: u64) -> Result<SolutionMap> {
        let mut crc2pts = HashMap::with_capacity(zip.len());
        for i in 0..zip.len() {
            let entry = zip.by_index_raw(i)?;
            if entry.size() == size {
                let (name, crc) = (entry.name(), entry.crc32());
                tracing::debug!("name={}, crc={:#x}", name, crc);

                crc2pts
                    .entry(crc)
                    .or_insert_with(|| (name.to_owned(), Mutex::default()));
            }
        }

        Ok(crc2pts)
    }

    fn spawn_tasks(ctx: &Arc<Context>) -> JoinSet<()> {
        let mut tasks = JoinSet::new();
        for c in ctx.alphabet.chars() {
            let ctx = ctx.clone();
            tasks.spawn(Self::brute(c, ctx));
        }

        tasks
    }
}

#[async_trait]
impl Command for ZipCrc {
    async fn execute(self: Box<Self>) -> Result<()> {
        let Self {
            file,
            size,
            alphabet,
        } = *self;
        let zip = ZipArchive::new(File::open(&file)?)?;
        let crc2pts = Self::init_buckets(zip, size)?;
        let ctx = Arc::new(Context::new(size, alphabet, crc2pts));

        let mut tasks = Self::spawn_tasks(&ctx);
        while let Some(result) = tasks.join_next().await {
            result?;
        }
        tracing::debug!(?ctx.crc2pts);

        let ctx = Arc::into_inner(ctx).ok_or(Error::ArcIntoInner)?;
        let crc2pts_sorted = ctx
            .crc2pts
            .into_iter()
            .map(|(crc, (name, pts))| (name, (crc, pts.into_inner())))
            .collect::<BTreeMap<_, _>>();
        crc2pts_sorted.iter().for_each(|(name, (crc, pts))| {
            tracing::info!("name={}, crc={:#x}, pts={:?}", name, crc, pts)
        });

        let pt = crc2pts_sorted
            .into_iter()
            .flat_map(|(_, (_, pts))| pts)
            .collect::<String>();
        tracing::info!(pt);

        Ok(())
    }
}

#[derive(Debug)]
struct Context {
    pub size: usize,
    pub alphabet: String,
    pub crc2pts: HashMap<u32, (String, Mutex<Vec<String>>)>,
}

impl Context {
    fn new(
        size: u64,
        alphabet: String,
        crc2pts: HashMap<u32, (String, Mutex<Vec<String>>)>,
    ) -> Self {
        Self {
            size: size.try_into().expect("Failed to convert u64 to usize."),
            alphabet,
            crc2pts,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tokio::sync::Mutex;

    use super::{Context, SolutionMap, ZipCrc};

    #[tokio::test]
    async fn test_brute() {
        let flag = "flag".to_owned();
        let crc = crc32fast::hash(flag.as_bytes());
        let mut crc2pts = SolutionMap::new();
        crc2pts.insert(
            crc,
            ("demo.txt".to_owned(), Mutex::<Vec<String>>::default()),
        );
        let alphabet = ('a'..'z').collect();
        let ctx = Arc::new(Context::new(4, alphabet, crc2pts));
        ZipCrc::brute('f', ctx.clone()).await;

        let Context { mut crc2pts, .. } = Arc::into_inner(ctx).unwrap();
        assert_eq!(crc2pts.remove(&crc).unwrap().1.into_inner(), vec![flag]);
    }
}
