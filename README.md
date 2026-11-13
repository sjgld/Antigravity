# Antigravity 解锁器 · 简体中文版

基于 [confeden/Antigravity](https://github.com/confeden/Antigravity) **v2.31.2** 的简体中文分支，由 sjgld 维护。中文修订版本为 **2.31.2_1**。

主界面、授权页、高级设置、功能说明、状态卡片、错误提示、操作日志、诊断报告和终端界面均使用简体中文。内置 Noto Sans SC 字体，在非中文系统上也能正常显示。产品名称、域名、命令、配置字段和协议标识保留原样。

这是第三方工具，用于调整 Antigravity 的客户端地区判断和连接线路，不提供模型额度，也不是 Google 官方软件。是否可用仍取决于账号、网络出口及 Google 服务状态。

## 下载与使用

前往 [Releases](https://github.com/sjgld/Antigravity/releases) 下载对应平台的中文便携版。

- **Windows**：下载 ZIP，解压后运行 `AG_2.31.2_1.exe`。
- **macOS（Apple Silicon）**：下载 `AG_2.31.2_1_macos.tar.gz`，解压后运行 `启动.command`。最低目标系统为 macOS 13，详见 [Mac 使用说明](macos/README.md)。

以下步骤适用于 Windows：

1. 安装 Google 官方 Antigravity。上游声明支持 Antigravity 2.31.1 及更新版本；中文分支基于 2.31.2 源码。
2. 以管理员身份运行解锁器。客户端补丁可以在用户权限下应用，DNS 规则和自动补丁后台组件需要管理员权限。
3. 输入适用于上游 2.31.2 的免费授权密钥。中文修订号不改变上游的版本密钥校验。
4. 点击 **全部启用**。应用补丁期间 Antigravity 可能暂时关闭。
5. 重新打开 Antigravity、登录账号并发送一条消息。只有检测到模型回复后，状态卡片才会显示 **连接正常**；“已全部启用”仅表示组件已配置。

可在主界面关闭对应开关撤销设置。诊断报告可通过 **保存诊断报告** 导出到桌面。

### 公开构建与作者发行版的区别

上游没有公开群组代理的私有构建配置、部分私有线路文件及服务端。本仓库的公开构建提供客户端补丁、DNS 线路、本地代理、自定义 HTTP 代理及自动补丁；**不包含作者的私有群组代理能力**。界面会显示相关提示。

使用自己的代理时，在高级设置填写 `主机:端口` 或 `用户名:密码@主机:端口`。目前支持 HTTP 代理，不支持直接填写 SOCKS 地址。

应用内更新检查指向本中文分支，不会将你引导回俄语发行版。中文版与上游使用相同的本机设置目录和后台组件，运行时请只保留一个版本。

## 本地构建

Windows 需要 Rust MSVC 工具链及 Visual Studio C++ 构建工具。

```powershell
git clone --branch zh-CN https://github.com/sjgld/Antigravity.git
cd Antigravity
python tools/check_localization.py
cargo test --locked --release
cargo build --locked --release
```

生成文件：`target/release/ag_unlocker.exe`。运行 `ag_unlocker.exe --tui` 可使用终端界面。

macOS 构建由 `.github/workflows/macos.yml` 在 Mac 环境完成，可使用 Apple Silicon 原生编译或 Intel 交叉编译，产物均为 ARM64。使用本地临时签名，未进行 Apple 公证。Linux 的界面文本同样已中文化，但本分支暂未发布 Linux 二进制包，平台运行方式见 [上游说明](README.ru.md)。

## 中文化维护

- 保持协议字段、二进制补丁特征、域名、环境变量及机器可读标识不变。
- `tools/check_localization.py` 检查残留俄语界面字符串；旧日志脱敏规则、俄语系统错误匹配和 Unicode 测试样本单独保留。
- 中文字符按双列宽度布局终端内容；诊断报告仍支持旧版俄语日志的地址脱敏。
- 原始说明保存在 [README.ru.md](README.ru.md)。版权及原始来源信息继续保留。

## 许可证与字体

应用使用上游的 [限制性许可证](LICENSE)，并非 MIT/Apache 等宽松许可证。公开仓库不意味着可以自由商业使用或继续发布修改版，后续使用和再分发应确认相应授权范围。

内置字体使用独立的 [SIL Open Font License 1.1](assets/fonts/OFL.txt)，来源和生成方式见 [字体说明](assets/fonts/README.md)。
