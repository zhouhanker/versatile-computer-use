# Windows 开发流程

更新：2026-09-26。这是本机 Windows 上改代码、测 Edge、更新扩展、再推 GitHub 的流程。已发布的浏览器桥仍是 **0.2.8**。不上 Chrome 网上应用店，也不上 Edge 加载项。不新开 GitHub Release。

## 测哪个浏览器

本机只测 Edge。Chrome 与 Edge 用同一套扩展和内核，Windows 上不另开 Chrome 复测，除非改动明确分浏览器。

不要点「允许调试」。不要移动系统光标。不要自动化微信。不要改 `~/.codex/computer-use/`。

## 扩展从哪里加载

开发时的源文件是仓库里的 `extension/`。浏览器不会读 Git 仓库，它只读「加载解压缩的扩展」指向的目录。

两种目录：

- 仓库的 `extension/`。`git pull` 之后，扩展页路径没变，再点一次重新加载。
- `%USERPROFILE%\.vcu\lens-extension`。`git pull` 不会改这个目录。只点重新加载，加载的仍是旧文件。

清单版本仍是 `0.2.8`。复制时指定来源，避免装到 `~/.local/share/vcu/extension` 里的旧包：

```powershell
vcu browser install-lens --from extension --reload
vcu browser ping --json
```

`--reload` 让已连接的 service worker 执行 `chrome.runtime.reload()`。worker 已经死掉时，Windows 没有 macOS 那种打开 `reload.html` 的兜底。先复制文件，再在 `edge://extensions` 点一次重新加载。

已经打开的网页不会自动换成新内容脚本。那些页要刷新，否则会报 `content lens is stale`。

`vcu self update` 和 Release `v0.2.8` 的 `vcu-lens-extension.zip` 只含已发布包。只在 `main` 上的提交不会通过这两条路径到达浏览器。

## 打开网页

`vcu browser open` 在当前窗口新建后台标签，放进折叠的紫色原生标签组「VCU」，同窗口复用，不接管已有用户组。不替换用户正在看的页面，也不聚焦窗口。`--background` 仍可写，但已经不是防抢焦点的开关。

2026-09-26 本机 Edge 复测：焦点停在用户当前页，`https://example.com` 进入 VCU 组且 `active=false`，测完已关。提交 `3e9bc57`。

`browser select` 和 `observe --tab` 在目标标签不可见时仍会切到该标签。这是截图需要当前标签，不是 `open` 路径。

## 测试

扩展改动先跑：

```powershell
node --test extension/tests/*.test.cjs
```

2026-09-26 是 47 项通过。再在 Edge 上打开一个 `https://example.com`，确认当前页没有被抢走，然后关掉测试标签。

Rust 改动才需要重建。本机工具链是 `cargo +stable-x86_64-pc-windows-gnu`，二进制在 `target/x86_64-pc-windows-gnu/debug/`。只改扩展时，已安装的 `%USERPROFILE%\.local\bin\vcu.exe` 可以仍是旧二进制；抢焦点的修复在已重载的扩展里。daemon 用户目录是 `%USERPROFILE%\.vcu`，扩展轮询 `http://127.0.0.1:17890`。

## 提交并推送

测试通过后再提交。只提交这次改到的产品文件。不要提交 AWR 本地运行目录，不要改写历史。作者是 `zhouhanker <zhouhanker@gmail.com>`。远端是 `git@github.com:zhouhanker/versatile-computer-use.git`。

```powershell
git push origin main
```

不要把停放的 `MAC-NEXT` 或 `FEISHU-001` 写成进行中。不要为扩展热修复新开 GitHub Release。

## macOS 同步同一提交

在 macOS 仓库里 `git pull`，使检出与 `origin/main` 一致。

扩展页指向仓库 `extension/` 时，路径没变就点重新加载。

扩展页指向 `~/.vcu/lens-extension` 时：

```bash
vcu browser install-lens --from ./extension --reload
vcu browser ping --json
```

不要点「允许调试」。
