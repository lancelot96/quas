# quas

`quas` 是用 Rust 语言编写的，用于 CTF 的命令行工具。如果觉得本工具还行，请点个 `STAR`。

## 简介

本着实践驱动学习的想法，一边学习 CTF 知识，一边将一些容易实现的功能汇总到一个程序中。部分功能参见示例用法，完整功能请通过直接执行程序查看子命令。

## 上手指南

### 依赖

#### 开发依赖

- Rust 环境
- pkgconf-pkg-config
- freetype2-devel
- fontconfig-devel

#### 运行依赖

- tshark

### 安装

#### 源码安装

```bash
git clone https://github.com/lancelot96/quas
cd quas
cargo build --release
```

#### 二进制安装

直接下载 Github Actions 编译好的二进制文件即可，目前仅支持 Linux 操作系统。

### 示例用法

直接运行 `quas` 即可查看帮助信息，如下所示：

```bash
Usage: quas [OPTIONS] <COMMAND>

Commands:
  pngcrc
  zipcrc
  base64steg
  behinder
  keytraffic
  mousetraffic
  imagesteg
  imageutil
  help          Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose...
  -h, --help        Print help
  -V, --version     Print version
```

多次使用 `-v` 选项可以调节日志等级，无该参数时日志等级为 `INFO`，出现一次为 `DEBUG`，出现两次为 `TRACE`，可以通过该功能打印更详细的日志排查错误。

#### PNG 图片宽高 CRC32 爆破

```bash
quas pngcrc --in test/crc-broken.png
2024-03-14T13:09:56.597078Z  INFO quas::png_crc: Read png with width(0x135), height(0xe8) and CRC(0x93cf1eca).
2024-03-14T13:09:56.597093Z  INFO quas::png_crc: Computed CRC is 0x1aae416.
2024-03-14T13:09:56.597194Z  INFO quas::png_crc: Found correct Width(0x3fe).
2024-03-14T13:09:56.597556Z  INFO quas::png_crc: Fixed png saved as ("crc-broken-fixed.png").
```

#### 加密 ZIP 文件内文本 CRC32 爆破

```bash
quas zipcrc --in test/4ByteDemo.zip --size 4
2024-03-14T13:10:34.582942Z  INFO quas::zip_crc: name=1, crc=0xce70d424, pts=["pass"]
2024-03-14T13:10:34.582970Z  INFO quas::zip_crc: name=2, crc=0xc3f17511, pts=["word"]
2024-03-14T13:10:34.582976Z  INFO quas::zip_crc: name=3, crc=0xf90c8a70, pts=[" is "]
2024-03-14T13:10:34.582981Z  INFO quas::zip_crc: name=4, crc=0xcb8ed73f, pts=["0day"]
2024-03-14T13:10:34.582984Z  INFO quas::zip_crc: name=5, crc=0x338d5bac, pts=["dog6"]
2024-03-14T13:10:34.582988Z  INFO quas::zip_crc: name=6, crc=0xa4ceedf0, pts=["yyds"]
2024-03-14T13:10:34.582995Z  INFO quas::zip_crc: pt="password is 0daydog6yyds"
```

#### 冰蝎加密流量解密

```bash
quas behinder --in test/beyond.pcapng -k fb59891768280222
2024-03-14T13:12:19.352988Z  INFO quas::behinder: file="behinder/343"
2024-03-14T13:12:19.353259Z  INFO quas::behinder: file="behinder/344.json"
2024-03-14T13:12:19.353478Z  INFO quas::behinder: file="behinder/345"
2024-03-14T13:12:19.358912Z  INFO quas::behinder: file="behinder/346.json"
2024-03-14T13:12:19.359193Z  INFO quas::behinder: file="behinder/403"
2024-03-14T13:12:19.359318Z  INFO quas::behinder: file="behinder/404.json"
2024-03-14T13:12:19.359469Z  INFO quas::behinder: file="behinder/459"
2024-03-14T13:12:19.359598Z  INFO quas::behinder: file="behinder/460.json"
2024-03-14T13:12:19.359837Z  INFO quas::behinder: file="behinder/809"
2024-03-14T13:12:19.359953Z  INFO quas::behinder: file="behinder/810.json"
```

#### USB 键盘流量提取

```bash
quas keytraffic --in test/keyboard.pcap
2024-03-14T13:12:50.590573Z  INFO quas::key_traffic: steg="flag{pr355_0nwards_a2fee6e0}"
```

#### USB 鼠标流量提取

```bash
quas mousetraffic --in test/mouse.pcap
2024-03-14T13:13:10.882108Z  INFO quas::mouse_traffic: Mouse trace saved as ("mouse.png").
```

#### 图片 LSB 隐写

```bash
quas imagesteg --in mouse.png -r1 -g1 -b1 -f aspect
2024-03-14T13:14:34.134105Z  INFO quas::image_steg: file="mouse.png" width=1920 height=1080
2024-03-14T13:14:34.186129Z  INFO quas::image_steg: file_path="mouse/[0, 0, 1, 0].aspect.png"
2024-03-14T13:14:34.235463Z  INFO quas::image_steg: file_path="mouse/[0, 1, 0, 0].aspect.png"
2024-03-14T13:14:34.281580Z  INFO quas::image_steg: file_path="mouse/[1, 0, 0, 0].aspect.png"
```

## 版本历史

- 0.2.1
  - 为 `imagesteg` 子命令添加 `-yXYo` 参数
- 0.2
  - 添加 `imagesteg` 子命令
  - 添加 `imageutil` 子命令
- 0.1
  - 添加 `pngcrc` 子命令
  - 添加 `zipcrc` 子命令
  - 添加 `base64steg` 子命令
  - 添加 `behinder` 子命令
  - 添加 `keytraffic` 子命令
  - 添加 `mousetraffic` 子命令
