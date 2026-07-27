use serde_json::{json, Value};
use crate::modules::fs::file::write_atomic;

// How a given agent's hook delivers our OSC 777 marker into the terminal.
#[derive(Clone, Copy)]
enum Delivery {
    // Claude returns the sequence via a `terminalSequence` JSON field (it lost
    // /dev/tty access in v2.1.139) and emits it in-band. Cross-platform.
    TerminalSequence,
    // Codex/Gemini hooks can't write to the terminal, so the hook command emits
    // the marker itself: to /dev/tty on Unix, via a CONOUT$ helper on Windows.
    Osc,
    Plugin,
    // Kimi Code：写入 ~/.kimi-code/config.toml 的 [[hooks]] 数组（非 JSON）。
    Toml,
}

struct AgentSpec {
    agent: &'static str,
    dir: &'static str,
    file: &'static str,
    events: &'static [(&'static str, &'static str)],
    matcher: bool,
    delivery: Delivery,
}

const AGENTS: &[AgentSpec] = &[
    AgentSpec {
        agent: "claude",
        dir: ".claude",
        file: "settings.json",
        events: &[
            ("UserPromptSubmit", "working"),
            ("Notification", "attention"),
            ("Stop", "finished"),
        ],
        matcher: false,
        delivery: Delivery::TerminalSequence,
    },
    AgentSpec {
        agent: "codex",
        dir: ".codex",
        file: "hooks.json",
        events: &[
            ("UserPromptSubmit", "working"),
            ("PermissionRequest", "attention"),
            ("Stop", "finished"),
        ],
        matcher: false,
        delivery: Delivery::Osc,
    },
    AgentSpec {
        agent: "gemini",
        dir: ".gemini",
        file: "settings.json",
        events: &[
            ("BeforeAgent", "working"),
            ("Notification", "attention"),
            ("AfterAgent", "finished"),
        ],
        matcher: true,
        delivery: Delivery::Osc,
    },
    AgentSpec {
        agent: "grok",
        dir: ".grok/hooks",
        file: "terax.json",
        events: &[
            ("UserPromptSubmit", "working"),
            // Grok 的 Notification 在回合结束时也会触发，不能当作「需要输入」。
            ("Stop", "finished"),
        ],
        matcher: false,
        delivery: Delivery::Osc,
    },
    AgentSpec {
        agent: "opencode",
        dir: ".config/opencode/plugins",
        file: "terax-agent-notifications.js",
        events: &[],
        matcher: false,
        delivery: Delivery::Plugin,
    },
    AgentSpec {
        agent: "kimi",
        dir: ".kimi-code",
        file: "config.toml",
        events: &[
            ("UserPromptSubmit", "working"),
            ("PermissionRequest", "attention"),
            ("Stop", "finished"),
        ],
        matcher: false,
        delivery: Delivery::Toml,
    },
];

// Substrings identifying a hook command as ours, across every form we've ever
// emitted (legacy /dev/tty Claude, current TerminalSequence, Osc, Windows
// helper). Used to prune our own groups before reinserting so installs are
// idempotent and migrate older markers.
const OWNED_MARKERS: [&str; 4] = [
    "notify;Terax;",
    "terax;notify",
    "__terax_notify",
    "terax_notify_",
];

/// 返回 Terax 管理的 OpenCode 全局通知插件源码。
fn opencode_plugin_source() -> &'static str {
    r#"const TERAX_OWNER = "Managed by Terax: agent notifications"
const emit = (event) => {
  if (process.env.TERAX_TERMINAL !== "1") return
  const marker = event === "attention"
    ? "\u001b]777;notify;Terax;opencode;attention\u0007"
    : "\u001b]777;notify;Terax;opencode;finished\u0007"
  process.stdout.write(marker)
}

export const TeraxAgentNotifications = async () => ({
  event: async ({ event }) => {
    if (event.type === "permission.asked") emit("attention")
    if (event.type === "session.idle") emit("finished")
  },
})
"#
}

/// 判断 OpenCode 插件文件是否可由 Terax 安全创建或更新。
fn can_write_opencode_plugin(existing: Option<&str>) -> bool {
    existing.is_none_or(|text| text.contains("Managed by Terax: agent notifications"))
}

/// 判断 OpenCode 插件是否包含 Terax 管理的完整完成通知逻辑。
fn opencode_plugin_installed(contents: &str) -> bool {
    contents.contains("Managed by Terax: agent notifications")
        && contents.contains("session.idle")
        && contents.contains("opencode;finished")
}

fn find(agent: &str) -> Result<&'static AgentSpec, String> {
    AGENTS
        .iter()
        .find(|s| s.agent == agent)
        .ok_or_else(|| format!("unknown agent {agent}"))
}

fn hook_command(spec: &AgentSpec, event: &str) -> String {
    match spec.delivery {
        // Claude 在 Unix 用 terminalSequence；Windows 上 hook 经 sh -c 执行，
        // 且 GUI 子系统无 /dev/tty，统一走 CONOUT$ 辅助入口。
        Delivery::TerminalSequence => {
            #[cfg(windows)]
            {
                osc_command(spec.agent, event)
            }
            #[cfg(not(windows))]
            {
                format!(
                    r#"[ -n "$TERAX_TERMINAL" ] && printf '{{"terminalSequence":"\\u001b]777;notify;Terax;{event}\\u0007"}}' || true"#
                )
            }
        }
        Delivery::Osc => osc_command(spec.agent, event),
        // Kimi 会把 hook stdout 拼进上下文；禁止输出 `{}` 之类的占位 JSON。
        Delivery::Toml => kimi_osc_command(spec.agent, event),
        Delivery::Plugin => unreachable!("OpenCode plugin does not use JSON hook commands"),
    }
}

// Marker to the tty, then `{}` on stdout: Codex/Gemini require a JSON no-op.
#[cfg(unix)]
fn osc_command(agent: &str, event: &str) -> String {
    format!(
        r#"[ -n "$TERAX_TERMINAL" ] && printf '\033]777;notify;Terax;{agent};{event}\007' > /dev/tty; printf '{{}}'"#
    )
}

/// Windows：写入无 shell 元字符的 `.cmd` 包装，供 Grok CreateProcess 直调。
/// 含空格/`>`/`;` 的一行命令会走 `sh -c`，GUI 子系统在重定向下常假报 exit 1。
#[cfg(windows)]
fn osc_command(agent: &str, event: &str) -> String {
    write_notify_cmd(agent, event, true).unwrap_or_else(|_| {
        // 回退仍尽量可用；状态检查会因路径不含针而显示未启用。
        format!(
            "{}/{}",
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| slash_path(&d.to_string_lossy())))
                .unwrap_or_else(|| ".".into()),
            notify_cmd_basename(agent, event)
        )
    })
}

/// Kimi hook 命令：与 Osc 相同传递 OSC，但不向 stdout 写 JSON（避免污染会话上下文）。
#[cfg(unix)]
fn kimi_osc_command(agent: &str, event: &str) -> String {
    format!(
        r#"[ -n "$TERAX_TERMINAL" ] && printf '\033]777;notify;Terax;{agent};{event}\007' > /dev/tty; cat >/dev/null || true"#
    )
}

#[cfg(windows)]
fn kimi_osc_command(agent: &str, event: &str) -> String {
    write_notify_cmd(agent, event, false).unwrap_or_else(|_| osc_command(agent, event))
}

#[cfg(windows)]
fn notify_cmd_basename(agent: &str, event: &str) -> String {
    format!("terax_notify_{agent}_{event}.cmd")
}

/// 在 terax.exe 同目录写入 `terax_notify_<agent>_<event>.cmd`，始终 `exit /b 0`。
#[cfg(windows)]
fn write_notify_cmd(agent: &str, event: &str, echo_json: bool) -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let dir = exe
        .parent()
        .ok_or_else(|| "terax.exe has no parent directory".to_string())?;
    let exe_name = exe
        .file_name()
        .ok_or_else(|| "terax.exe has no file name".to_string())?
        .to_string_lossy();
    let path = dir.join(notify_cmd_basename(agent, event));
    let echo_line = if echo_json { "echo {}\r\n" } else { "" };
    let body = format!(
        "@echo off\r\nset TERAX_TERMINAL=1\r\n\"%~dp0{exe_name}\" __terax_notify {agent} {event} >nul 2>&1\r\n{echo_line}exit /b 0\r\n"
    );
    std::fs::write(&path, body.as_bytes()).map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(slash_path(&path.to_string_lossy()))
}

#[cfg(windows)]
#[allow(dead_code)]
fn current_exe_slash() -> String {
    std::env::current_exe()
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| "terax.exe".to_string())
}

/// 把路径分隔符规范成 `/`，便于在 JSON 原文中比对安装版/开发版可执行文件。
#[cfg(windows)]
fn slash_path(path: &str) -> String {
    path.replace('\\', "/")
}

// The stable substring that proves a given (agent, event) hook is installed.
// Kept in sync with hook_command so status reflects what enable writes.
fn status_needle(spec: &AgentSpec, event: &str) -> String {
    match spec.delivery {
        Delivery::TerminalSequence => {
            #[cfg(windows)]
            {
                notify_cmd_basename(spec.agent, event)
            }
            #[cfg(not(windows))]
            {
                format!("notify;Terax;{event}")
            }
        }
        Delivery::Osc | Delivery::Toml => {
            #[cfg(unix)]
            {
                format!("notify;Terax;{};{event}", spec.agent)
            }
            #[cfg(windows)]
            {
                notify_cmd_basename(spec.agent, event)
            }
        }
        Delivery::Plugin => String::new(),
    }
}

/// Windows：事件针已匹配后，再确认命令指向当前有效的 terax.exe 目录。
#[cfg(windows)]
fn windows_exe_matches(content: &str) -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let Some(dir) = exe.parent() else {
        return false;
    };
    slash_path(content).contains(&slash_path(&dir.to_string_lossy()))
}

fn is_ours(group: &Value) -> bool {
    group
        .get("hooks")
        .and_then(Value::as_array)
        .is_some_and(|hs| {
            hs.iter().any(|h| {
                h.get("command")
                    .and_then(Value::as_str)
                    .is_some_and(|c| OWNED_MARKERS.iter().any(|m| c.contains(m)))
            })
        })
}

// A group with no hooks is inert cruft (e.g. left behind when someone deletes
// our command but not its wrapper). Drop it so the file stays clean.
fn is_empty_group(group: &Value) -> bool {
    group
        .get("hooks")
        .and_then(Value::as_array)
        .is_none_or(|hs| hs.is_empty())
}

fn merge_hooks(mut root: Value, spec: &AgentSpec) -> Value {
    if !root.is_object() {
        root = json!({});
    }
    let obj = root.as_object_mut().unwrap();
    let hooks = obj.entry("hooks").or_insert_with(|| json!({}));
    if !hooks.is_object() {
        *hooks = json!({});
    }
    let hooks = hooks.as_object_mut().unwrap();

    // 先剥离所有事件中的 Terax hook，再按当前 spec 重装（避免已移除的事件残留）。
    for (_event, value) in hooks.iter_mut() {
        if let Some(arr) = value.as_array_mut() {
            arr.retain(|group| !is_ours(group) && !is_empty_group(group));
        }
    }

    for (event, marker) in spec.events {
        let arr = hooks.entry(*event).or_insert_with(|| json!([]));
        if !arr.is_array() {
            *arr = json!([]);
        }
        let arr = arr.as_array_mut().unwrap();
        let mut group = json!({
            "hooks": [ { "type": "command", "command": hook_command(spec, marker) } ]
        });
        if spec.matcher {
            group["matcher"] = json!("*");
        }
        arr.push(group);
    }
    root
}

/// 转义 TOML 基本字符串（双引号包裹）。
fn escape_toml_basic_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}

fn is_owned_hook_command(command: &str) -> bool {
    OWNED_MARKERS.iter().any(|m| command.contains(m))
}

/// 判断一行是否为 TOML 表头（`[section]` 或 `[[array]]`）。
fn is_toml_table_header(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with('[')
}

/// 从 config.toml 中移除 Terax 管理的 `[[hooks]]` 表，保留用户与其它工具的 hook。
fn strip_owned_kimi_hooks(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    let mut lines = content.lines().peekable();
    while let Some(line) = lines.next() {
        if line.trim() == "[[hooks]]" {
            let mut block = vec![line.to_string()];
            while let Some(next) = lines.peek() {
                if is_toml_table_header(next) {
                    break;
                }
                block.push(lines.next().unwrap().to_string());
            }
            let owned = block.iter().any(|l| {
                let t = l.trim_start();
                t.starts_with("command")
                    && t.contains('=')
                    && is_owned_hook_command(t)
            });
            if owned {
                continue;
            }
            for l in &block {
                out.push_str(l);
                out.push('\n');
            }
            continue;
        }
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// 合并 Kimi Code 的 `[[hooks]]`：先剥离旧 Terax 项，再追加 working/attention/finished。
fn merge_kimi_hooks(content: &str, spec: &AgentSpec) -> String {
    let mut out = strip_owned_kimi_hooks(content);
    // 去掉末尾多余空行，再统一追加。
    while out.ends_with("\n\n") {
        out.pop();
    }
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    if !out.is_empty() {
        out.push('\n');
    }
    for (event, marker) in spec.events {
        let cmd = escape_toml_basic_string(&hook_command(spec, marker));
        out.push_str("[[hooks]]\n");
        out.push_str(&format!("event = \"{event}\"\n"));
        out.push_str(&format!("command = \"{cmd}\"\n"));
        out.push_str("timeout = 10\n\n");
    }
    out
}

fn existing_config(contents: Option<&str>, path: &std::path::Path) -> Result<Value, String> {
    match contents {
        Some(s) if !s.trim().is_empty() => serde_json::from_str::<Value>(s).map_err(|e| {
            format!("{} is not valid JSON ({e}); refusing to overwrite", path.display())
        }),
        _ => Ok(json!({})),
    }
}

fn settings_path(spec: &AgentSpec) -> Result<std::path::PathBuf, String> {
    if spec.agent == "kimi" {
        return kimi_settings_path();
    }
    Ok(dirs::home_dir()
        .ok_or_else(|| "could not resolve home dir".to_string())?
        .join(spec.dir)
        .join(spec.file))
}

/// Kimi Code 配置路径：优先 `KIMI_CODE_HOME`，否则 `~/.kimi-code/config.toml`。
fn kimi_settings_path() -> Result<std::path::PathBuf, String> {
    if let Ok(home) = std::env::var("KIMI_CODE_HOME") {
        let trimmed = home.trim();
        if !trimmed.is_empty() {
            return Ok(std::path::PathBuf::from(trimmed).join("config.toml"));
        }
    }
    Ok(dirs::home_dir()
        .ok_or_else(|| "could not resolve home dir".to_string())?
        .join(".kimi-code")
        .join("config.toml"))
}

#[tauri::command]
pub fn agent_enable_hooks(agent: String) -> Result<(), String> {
    let spec = find(&agent)?;
    let path = settings_path(spec)?;
    let dir = path.parent().unwrap();
    std::fs::create_dir_all(dir).map_err(|e| format!("create {}: {e}", dir.display()))?;

    if matches!(spec.delivery, Delivery::Plugin) {
        let existing = match std::fs::read_to_string(&path) {
            Ok(contents) => Some(contents),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
            Err(e) => return Err(format!("read {}: {e}", path.display())),
        };
        if !can_write_opencode_plugin(existing.as_deref()) {
            return Err(format!(
                "{} is not managed by Terax; refusing to overwrite",
                path.display()
            ));
        }
        write_atomic(&path, opencode_plugin_source().as_bytes())
            .map_err(|e| format!("write {}: {e}", path.display()))?;
        return Ok(());
    }

    if matches!(spec.delivery, Delivery::Toml) {
        let existing = match std::fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(e) => return Err(format!("read {}: {e}", path.display())),
        };
        let merged = merge_kimi_hooks(&existing, spec);
        write_atomic(&path, merged.as_bytes())
            .map_err(|e| format!("write {}: {e}", path.display()))?;
        return Ok(());
    }

    let existing = match std::fs::read_to_string(&path) {
        Ok(s) => existing_config(Some(&s), &path)?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => json!({}),
        Err(e) => return Err(format!("read {}: {e}", path.display())),
    };

    let merged = merge_hooks(existing, spec);
    let out = serde_json::to_string_pretty(&merged).map_err(|e| e.to_string())?;

    write_atomic(&path, out.as_bytes()).map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(())
}

// The raw OSC 777 bytes the detector parses. Kept in one place so the Windows
// CONOUT$ path can't drift from what the Unix /dev/tty hook emits.
#[cfg(any(windows, test))]
fn conout_marker(agent: &str, event: &str) -> String {
    format!("\x1b]777;notify;Terax;{agent};{event}\x07")
}

// Windows has no /dev/tty: the hook calls `terax.exe __terax_notify ...` and we
// write the marker into the ConPTY console. GUI-subsystem release inherits no
// console, so attach to the hook runner's first.
#[cfg(windows)]
pub fn emit_conout_marker(agent: &str, event: &str) {
    use std::io::Write;
    use windows_sys::Win32::Foundation::{FALSE, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Console::{
        AttachConsole, FreeConsole, GetStdHandle, WriteConsoleA, ATTACH_PARENT_PROCESS,
        STD_OUTPUT_HANDLE,
    };

    if std::env::var_os("TERAX_TERMINAL").is_none() {
        return;
    }

    let marker = conout_marker(agent, event);
    let bytes = marker.as_bytes();

    // Grok/Codex 常经 `sh -c` 起 hook：直接父进程可能没有控制台。
    // 沿祖先链 AttachConsole，直到写到承载 ConPTY 的 shell（如 pwsh）。
    let mut targets: Vec<u32> = vec![ATTACH_PARENT_PROCESS];
    targets.extend(ancestor_pids(8));

    for pid in targets {
        unsafe {
            let _ = FreeConsole();
            if AttachConsole(pid) == FALSE {
                continue;
            }
            let handle = GetStdHandle(STD_OUTPUT_HANDLE);
            if !handle.is_null() && handle != INVALID_HANDLE_VALUE {
                let mut written = 0u32;
                if WriteConsoleA(
                    handle,
                    bytes.as_ptr().cast(),
                    bytes.len() as u32,
                    &mut written,
                    std::ptr::null_mut(),
                ) != FALSE
                {
                    let _ = FreeConsole();
                    return;
                }
            }
            // 已附着但 WriteConsole 失败时，尝试 CONOUT$。
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open("CONOUT$")
            {
                if f.write_all(bytes).is_ok() {
                    let _ = FreeConsole();
                    return;
                }
            }
            let _ = FreeConsole();
        }
    }
}

/// 返回从 `start_pid` 起的祖先链（含自身，近→远）。
#[cfg(windows)]
pub fn process_ancestor_chain(start_pid: u32, max: usize) -> Vec<u32> {
    use std::mem::{size_of, zeroed};
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32First, Process32Next, PROCESSENTRY32,
        TH32CS_SNAPPROCESS,
    };

    let mut out = Vec::with_capacity(max.saturating_add(1));
    out.push(start_pid);
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return out;
        }
        let mut entry: PROCESSENTRY32 = zeroed();
        entry.dwSize = size_of::<PROCESSENTRY32>() as u32;

        let mut parents: std::collections::HashMap<u32, u32> = std::collections::HashMap::new();
        if Process32First(snapshot, &mut entry) != 0 {
            loop {
                parents.insert(entry.th32ProcessID, entry.th32ParentProcessID);
                if Process32Next(snapshot, &mut entry) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snapshot);

        let mut pid = start_pid;
        for _ in 0..max {
            let Some(&ppid) = parents.get(&pid) else {
                break;
            };
            if ppid == 0 || ppid == pid || out.contains(&ppid) {
                break;
            }
            out.push(ppid);
            pid = ppid;
        }
    }
    out
}

/// 返回当前进程的祖先 PID（近→远），用于定位真正持有 ConPTY 控制台的进程。
#[cfg(windows)]
fn ancestor_pids(max: usize) -> Vec<u32> {
    use windows_sys::Win32::System::Threading::GetCurrentProcessId;
    let mut chain = process_ancestor_chain(unsafe { GetCurrentProcessId() }, max);
    if !chain.is_empty() {
        chain.remove(0); // 去掉自身
    }
    chain
}

const NOTIFY_PIPE_NAME: &str = r"\\.\pipe\terax-agent-notify";

/// Hook 辅助进程：经命名管道把事件交给正在运行的 Terax（不依赖 ConPTY 写入）。
#[cfg(windows)]
pub fn send_notify_ipc(agent: &str, event: &str) -> bool {
    use std::fs::OpenOptions;
    use std::io::Write;
    use windows_sys::Win32::System::Pipes::WaitNamedPipeW;
    use windows_sys::Win32::System::Threading::GetCurrentProcessId;

    let pid = unsafe { GetCurrentProcessId() };
    // 进程树校验必须在助手进程内完成：本进程写完管道立即退出，主进程在独立
    // 线程里再做 Toolhelp 快照时通常已查不到任何映像名，claude 会被「断链
    // 拒绝」整批丢掉（日志表现为 `reject unverified claude claim names=[]`）。
    let tree = u8::from(caller_matches_agent(agent, pid));
    // 始终带上调用方 pid，便于主进程校验「钩子 agent」与真实进程树是否一致。
    // 若 shell 继承了 TERAX_PTY_ID，再附带 pty 以便精确路由。
    let pty = std::env::var("TERAX_PTY_ID").unwrap_or_default();
    let pty = pty.trim();
    let payload = if pty.is_empty() {
        format!("{agent} {event} pid:{pid} tree:{tree}\n")
    } else {
        format!("{agent} {event} pty:{pty} pid:{pid} tree:{tree}\n")
    };
    let wide: Vec<u16> = NOTIFY_PIPE_NAME
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();

    // Grok 会同时跑 ~/.claude/settings 与 ~/.grok/hooks，两条 Stop 几乎并发；
    // 多试几次，避免单实例管道忙时误走 CONOUT$ 把 stdout 弄乱、被标成 exit 1。
    for _ in 0..8 {
        unsafe {
            let _ = WaitNamedPipeW(wide.as_ptr(), 200);
        }
        if let Ok(mut file) = OpenOptions::new().write(true).open(NOTIFY_PIPE_NAME) {
            if file.write_all(payload.as_bytes()).is_ok() {
                return true;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(30));
    }
    false
}

/// 解析 hook IPC 报文：`agent event [pty:ID] pid:PID [tree:0|1]`
/// （兼容旧 `agent event PID` / 仅 pty / 不带 tree 的报文）。
/// `tree` 是助手进程在自身存活时算出的进程树校验结论。
#[cfg(any(windows, test))]
fn parse_notify_ipc(
    line: &str,
) -> Option<(&str, &str, NotifyTarget, Option<u32>, Option<bool>)> {
    let mut parts = line.split_whitespace();
    let agent = parts.next()?;
    let event = parts.next()?;
    if agent.is_empty() || event.is_empty() {
        return None;
    }

    let mut pty: Option<u32> = None;
    let mut pid: Option<u32> = None;
    let mut tree: Option<bool> = None;
    for token in parts {
        if let Some(id) = token.strip_prefix("pty:") {
            pty = Some(id.parse().ok()?);
        } else if let Some(id) = token.strip_prefix("pid:") {
            pid = Some(id.parse().ok()?);
        } else if let Some(flag) = token.strip_prefix("tree:") {
            tree = Some(match flag {
                "1" => true,
                "0" => false,
                _ => return None,
            });
        } else if let Ok(id) = token.parse::<u32>() {
            // 旧格式：`agent event PID`
            pid = Some(id);
        } else {
            return None;
        }
    }

    let target = if let Some(id) = pty {
        NotifyTarget::Pty(id)
    } else if let Some(id) = pid {
        NotifyTarget::Pid(id)
    } else {
        return None;
    };
    Some((agent, event, target, pid, tree))
}

#[cfg(any(windows, test))]
#[derive(Debug, PartialEq, Eq)]
enum NotifyTarget {
    Pty(u32),
    Pid(u32),
}

/// 主进程：监听 hook 命名管道，把事件路由到对应 PTY 会话。
#[cfg(windows)]
pub fn start_notify_listener(app: tauri::AppHandle) {
    use std::thread;
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE, FALSE};
    use windows_sys::Win32::Storage::FileSystem::PIPE_ACCESS_INBOUND;
    use windows_sys::Win32::System::Pipes::{
        ConnectNamedPipe, CreateNamedPipeW, PIPE_READMODE_BYTE, PIPE_TYPE_BYTE,
        PIPE_UNLIMITED_INSTANCES, PIPE_WAIT, PIPE_REJECT_REMOTE_CLIENTS,
    };

    thread::Builder::new()
        .name("terax-agent-notify".into())
        .spawn(move || {
            let wide: Vec<u16> = NOTIFY_PIPE_NAME
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            loop {
                // 允许多实例：Grok 会并发触发 Claude settings + 自有 hooks。
                let handle = unsafe {
                    CreateNamedPipeW(
                        wide.as_ptr(),
                        PIPE_ACCESS_INBOUND,
                        PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
                        PIPE_UNLIMITED_INSTANCES,
                        0,
                        256,
                        0,
                        std::ptr::null_mut(),
                    )
                };
                if handle == INVALID_HANDLE_VALUE {
                    log::warn!("agent notify pipe create failed");
                    thread::sleep(std::time::Duration::from_millis(500));
                    continue;
                }
                let connected = unsafe { ConnectNamedPipe(handle, std::ptr::null_mut()) } != FALSE
                    || {
                        use windows_sys::Win32::Foundation::GetLastError;
                        unsafe { GetLastError() == 535 } // ERROR_PIPE_CONNECTED
                    };
                if !connected {
                    unsafe {
                        CloseHandle(handle);
                    }
                    // 避免创建失败时忙等占满 CPU（会导致界面点击/滚动卡死）。
                    thread::sleep(std::time::Duration::from_millis(50));
                    continue;
                }

                let app = app.clone();
                // HANDLE 非 Send：经 isize 传到工作线程。
                let handle_bits = handle as isize;
                thread::Builder::new()
                    .name("terax-agent-notify-client".into())
                    .spawn(move || {
                        handle_notify_client(app, handle_bits as _);
                    })
                    .ok();
            }
        })
        .expect("spawn agent notify listener");
}

/// 处理单个 hook 客户端连接（在独立线程，避免堵死下一条 Stop）。
#[cfg(windows)]
fn handle_notify_client(app: tauri::AppHandle, handle: windows_sys::Win32::Foundation::HANDLE) {
    use tauri::Manager;
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::Storage::FileSystem::ReadFile;
    use windows_sys::Win32::System::Pipes::DisconnectNamedPipe;

    let mut buf = [0u8; 256];
    let mut n = 0u32;
    let ok = unsafe {
        ReadFile(
            handle,
            buf.as_mut_ptr().cast(),
            buf.len() as u32,
            &mut n,
            std::ptr::null_mut(),
        )
    } != windows_sys::Win32::Foundation::FALSE;
    unsafe {
        let _ = DisconnectNamedPipe(handle);
        CloseHandle(handle);
    }
    if !ok || n == 0 {
        return;
    }
    let line = String::from_utf8_lossy(&buf[..n as usize]);
    let Some((agent, event, target, caller_pid, tree)) = parse_notify_ipc(line.trim()) else {
        log::debug!("agent notify: bad payload {:?}", line.trim());
        return;
    };

    // Grok 兼容加载 ~/.claude/settings.json：Claude 钩子会在 Grok 回合里误报。
    // 用调用方进程树校验 agent，拒绝明显串台的通知。助手进程已把结论算在报文里
    // （它退出后这里再查进程树只会得到空结果），仅旧报文才回退到本地重算。
    let matches = match tree {
        Some(ok) => ok,
        None => caller_pid.is_none_or(|pid| caller_matches_agent(agent, pid)),
    };
    if !matches {
        log::info!(
            "agent notify: ignore mismatched agent={agent} event={event} pid={caller_pid:?}"
        );
        return;
    }

    let Some(state) = app.try_state::<crate::modules::pty::PtyState>() else {
        log::warn!("agent notify: PtyState unavailable");
        return;
    };
    let id = match target {
        NotifyTarget::Pty(pty_id) if state.has_session(pty_id) => Some(pty_id),
        NotifyTarget::Pty(pty_id) => {
            log::debug!("agent notify: stale pty id={pty_id}");
            None
        }
        NotifyTarget::Pid(pid) => state.find_id_for_descendant(pid),
    };
    let Some(id) = id else {
        log::info!(
            "agent notify: no pty for {:?} agent={agent} event={event}",
            target
        );
        return;
    };
    dispatch_hook_signal(&app, id, agent, event);
}

/// 钩子声明的 agent 是否与调用进程树相符（防 Grok 误跑 Claude settings hooks）。
///
/// 重要：Grok 的 hook runner 经常在进程树上断链（只剩 cmd/terax 助手），
/// 若「看不到就拒绝」会把真正的 grok finished 误杀，活动任务卡在「工作中」。
/// 因此对 grok 等：能认出声明的 agent → 通过；树空 → 放行；明确看到其它 agent 才拒绝。
///
/// 例外：`claude` 声明在断链时一律拒绝。Grok 会兼容加载 `~/.claude/settings.json`，
/// 其 Stop/Notification 会在 Grok 回合里冒充 Claude；而真正的 Claude CLI 进程树
/// 几乎总能看到 `claude`，不会误伤。
#[cfg(windows)]
fn caller_matches_agent(agent: &str, pid: u32) -> bool {
    let chain = process_ancestor_chain(pid, 24);
    let names = process_image_names(&chain);
    let paths = process_image_paths(&chain);

    if tree_has_agent(agent, &names, &paths) {
        return true;
    }

    let others = ["claude", "grok", "codex", "gemini", "kimi", "opencode"];
    let saw_other = others
        .iter()
        .any(|&a| a != agent && tree_has_agent(a, &names, &paths));
    if saw_other {
        log::debug!(
            "agent notify: tree has other agent; reject claim={agent} names={names:?} paths={paths:?}"
        );
        return false;
    }
    // Claude：断链不放行（防 Grok 串台）。其它 agent：断链放行以免漏掉 finished。
    if agent == "claude" {
        log::info!(
            "agent notify: reject unverified claude claim pid={pid} names={names:?}"
        );
        return false;
    }
    true
}

#[cfg(windows)]
fn tree_has_agent(agent: &str, names: &[String], paths: &[String]) -> bool {
    let tokens = agent_exe_tokens(agent);
    if names
        .iter()
        .any(|name| exe_matches_agent_tokens(name, tokens))
    {
        return true;
    }
    paths.iter().any(|p| path_matches_agent(p, agent))
}

/// 各 agent 在进程树中应出现的可执行名片段（小写、无 .exe）。
#[cfg(any(windows, test))]
fn agent_exe_tokens(agent: &str) -> &'static [&'static str] {
    match agent {
        "claude" => &["claude"],
        "grok" => &["grok"],
        "codex" => &["codex"],
        "gemini" => &["gemini"],
        "kimi" => &["kimi"],
        "opencode" => &["opencode"],
        _ => &[],
    }
}

#[cfg(any(windows, test))]
fn exe_matches_agent_tokens(exe_name: &str, tokens: &[&str]) -> bool {
    let base = exe_name
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(exe_name)
        .to_ascii_lowercase();
    let base = base.strip_suffix(".exe").unwrap_or(&base);
    tokens
        .iter()
        .any(|t| base == *t || base.starts_with(&format!("{t}-")))
}

/// 用完整路径识别（例如 Grok 的 `~\.grok\bin\agent.exe`）。
#[cfg(any(windows, test))]
fn path_matches_agent(path: &str, agent: &str) -> bool {
    let p = path.replace('/', "\\").to_ascii_lowercase();
    match agent {
        "grok" => p.contains("\\.grok\\"),
        "claude" => p.contains("\\claude") || p.contains("anthropic"),
        "codex" => p.contains("\\codex"),
        "gemini" => p.contains("\\gemini"),
        "kimi" => p.contains("\\kimi"),
        "opencode" => p.contains("\\opencode"),
        _ => false,
    }
}

/// 查询一组 PID 的映像名（失败则跳过该 PID）。
#[cfg(windows)]
fn process_image_names(pids: &[u32]) -> Vec<String> {
    use std::mem::{size_of, zeroed};
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32First, Process32Next, PROCESSENTRY32,
        TH32CS_SNAPPROCESS,
    };

    let want: std::collections::HashSet<u32> = pids.iter().copied().collect();
    let mut out = Vec::new();
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return out;
        }
        let mut entry: PROCESSENTRY32 = zeroed();
        entry.dwSize = size_of::<PROCESSENTRY32>() as u32;
        if Process32First(snapshot, &mut entry) != 0 {
            loop {
                if want.contains(&entry.th32ProcessID) {
                    let raw = &entry.szExeFile;
                    let len = raw.iter().position(|&c| c == 0).unwrap_or(raw.len());
                    let name = String::from_utf8_lossy(
                        &raw[..len]
                            .iter()
                            .map(|&c| c as u8)
                            .collect::<Vec<_>>(),
                    )
                    .into_owned();
                    if !name.is_empty() {
                        out.push(name);
                    }
                }
                if Process32Next(snapshot, &mut entry) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snapshot);
    }
    out
}

/// 查询完整映像路径（用于识别 `~\.grok\bin\agent.exe` 等）。
#[cfg(windows)]
fn process_image_paths(pids: &[u32]) -> Vec<String> {
    use windows_sys::Win32::Foundation::{CloseHandle, FALSE};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    let mut out = Vec::new();
    for &pid in pids {
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, pid);
            if handle.is_null() {
                continue;
            }
            let mut buf = [0u16; 512];
            let mut size = buf.len() as u32;
            let ok = QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut size) != FALSE;
            CloseHandle(handle);
            if !ok || size == 0 {
                continue;
            }
            out.push(String::from_utf16_lossy(&buf[..size as usize]));
        }
    }
    out
}

#[cfg(windows)]
fn dispatch_hook_signal(app: &tauri::AppHandle, id: u32, agent: &str, event: &str) {
    use crate::modules::pty::agent_detect::{AgentSignal, Transition};
    use tauri::Emitter;

    // 与 AgentDetector::ensure_armed 对齐：先保证前端有 session。
    let _ = app.emit(
        "terax:agent-signal",
        Transition::Started {
            agent: agent.to_string(),
        }
        .into_signal(id),
    );
    let kind = match event {
        "working" => "working",
        "attention" => "attention",
        "finished" => "finished",
        _ => return,
    };
    let _ = app.emit(
        "terax:agent-signal",
        AgentSignal {
            id,
            kind,
            agent: None,
        },
    );
}

#[tauri::command]
pub fn agent_hooks_status(agent: String) -> bool {
    let Ok(spec) = find(&agent) else {
        return false;
    };
    let Some(content) = settings_path(spec)
        .ok()
        .and_then(|p| std::fs::read_to_string(p).ok())
    else {
        return false;
    };
    if matches!(spec.delivery, Delivery::Plugin) {
        return opencode_plugin_installed(&content);
    }
    let events_ok = spec
        .events
        .iter()
        .all(|(_, m)| content.contains(&status_needle(spec, m)));
    if !events_ok {
        return false;
    }
    // Windows Osc / Claude / Kimi：必须指向当前有效可执行文件，避免安装目录变更后假阳性。
    #[cfg(windows)]
    if !matches!(spec.delivery, Delivery::Plugin) && !windows_exe_matches(&content) {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(agent: &str) -> &'static AgentSpec {
        find(agent).unwrap()
    }

    fn hook_count(root: &Value, event: &str) -> usize {
        root["hooks"][event].as_array().map_or(0, Vec::len)
    }

    fn command(root: &Value, event: &str, idx: usize) -> String {
        root["hooks"][event][idx]["hooks"][0]["command"]
            .as_str()
            .unwrap()
            .to_string()
    }

    #[test]
    fn claude_adds_all_event_hooks_to_empty_config() {
        let out = merge_hooks(json!({}), spec("claude"));
        assert_eq!(hook_count(&out, "UserPromptSubmit"), 1);
        assert_eq!(hook_count(&out, "Notification"), 1);
        assert_eq!(hook_count(&out, "Stop"), 1);
        #[cfg(not(windows))]
        {
            assert!(command(&out, "Notification", 0).contains("notify;Terax;attention"));
            assert!(command(&out, "Stop", 0).contains("notify;Terax;finished"));
            assert!(command(&out, "UserPromptSubmit", 0).contains("notify;Terax;working"));
            assert!(command(&out, "Stop", 0).contains("terminalSequence"));
            assert!(!command(&out, "Stop", 0).contains("/dev/tty"));
        }
        #[cfg(windows)]
        {
            // Windows：旁路 .cmd，避免 Grok sh -c + GUI 假 exit 1。
            assert!(
                command(&out, "Stop", 0).contains("terax_notify_claude_finished.cmd"),
                "{}",
                command(&out, "Stop", 0)
            );
            assert!(!command(&out, "Stop", 0).contains("terminalSequence"));
        }
    }

    #[test]
    fn is_idempotent_per_agent() {
        for agent in ["claude", "codex", "gemini"] {
            let s = spec(agent);
            let once = merge_hooks(json!({}), s);
            let twice = merge_hooks(once.clone(), s);
            assert_eq!(once, twice, "{agent} not idempotent");
        }
    }

    #[test]
    fn conout_marker_matches_detector_format() {
        // Exactly the bytes pty/agent_detect parses (ESC ] 777 ; ... BEL).
        assert_eq!(
            conout_marker("gemini", "attention"),
            "\u{1b}]777;notify;Terax;gemini;attention\u{7}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn codex_emits_four_field_dev_tty_marker() {
        let out = merge_hooks(json!({}), spec("codex"));
        assert_eq!(hook_count(&out, "UserPromptSubmit"), 1);
        assert_eq!(hook_count(&out, "PermissionRequest"), 1);
        assert_eq!(hook_count(&out, "Stop"), 1);
        let stop = command(&out, "Stop", 0);
        assert!(stop.contains("notify;Terax;codex;finished"));
        assert!(stop.contains("> /dev/tty"));
        // Codex Stop rejects empty/non-JSON stdout; the hook must emit a no-op.
        assert!(stop.contains("printf '{}'"));
        assert!(!stop.contains("terminalSequence"));
    }

    #[cfg(unix)]
    #[test]
    fn gemini_uses_matcher_and_named_marker() {
        let out = merge_hooks(json!({}), spec("gemini"));
        assert_eq!(out["hooks"]["BeforeAgent"][0]["matcher"], "*");
        assert!(command(&out, "AfterAgent", 0).contains("notify;Terax;gemini;finished"));
        assert!(command(&out, "Notification", 0).contains("notify;Terax;gemini;attention"));
    }

    #[test]
    fn migrates_legacy_dev_tty_hook() {
        let legacy = json!({
            "hooks": {
                "Notification": [
                    { "hooks": [ {
                        "type": "command",
                        "command": "[ -n \"$TERAX_TERMINAL\" ] && printf '\\033]777;terax;notify\\033\\\\' > /dev/tty || true"
                    } ] }
                ]
            }
        });
        let out = merge_hooks(legacy, spec("claude"));
        assert_eq!(hook_count(&out, "Notification"), 1);
        assert!(!command(&out, "Notification", 0).contains("/dev/tty"));
        #[cfg(not(windows))]
        assert!(command(&out, "Notification", 0).contains("terminalSequence"));
        #[cfg(windows)]
        assert!(command(&out, "Notification", 0).contains("terax_notify_claude_attention.cmd"));
    }

    #[test]
    fn preserves_unrelated_settings_and_foreign_hooks() {
        let input = json!({
            "permissions": { "allow": ["Bash"] },
            "hooks": {
                "Notification": [
                    { "hooks": [ { "type": "command", "command": "say hi" } ] }
                ]
            }
        });
        let out = merge_hooks(input, spec("claude"));
        assert_eq!(out["permissions"]["allow"][0], "Bash");
        assert_eq!(hook_count(&out, "Notification"), 2);
        assert_eq!(command(&out, "Notification", 0), "say hi");
    }

    #[test]
    fn replaces_non_object_root() {
        let out = merge_hooks(json!("garbage"), spec("codex"));
        assert_eq!(hook_count(&out, "Stop"), 1);
    }

    #[test]
    fn prunes_empty_groups_and_collapses_duplicates() {
        let input = json!({
            "hooks": {
                "Notification": [
                    { "hooks": [] },
                    { "hooks": [ { "type": "command", "command": hook_command(spec("claude"), "attention") } ] }
                ]
            }
        });
        let out = merge_hooks(input, spec("claude"));
        assert_eq!(hook_count(&out, "Notification"), 1);
        #[cfg(not(windows))]
        assert!(command(&out, "Notification", 0).contains("notify;Terax;attention"));
        #[cfg(windows)]
        assert!(command(&out, "Notification", 0).contains("terax_notify_claude_attention.cmd"));
    }

    #[test]
    fn existing_config_absent_or_empty_starts_fresh() {
        let p = std::path::Path::new("/x/settings.json");
        assert_eq!(existing_config(None, p).unwrap(), json!({}));
        assert_eq!(existing_config(Some("   \n"), p).unwrap(), json!({}));
    }

    #[test]
    fn existing_config_refuses_to_clobber_invalid_json() {
        let p = std::path::Path::new("/x/settings.json");
        assert!(existing_config(Some("{ not json,"), p).is_err());
        assert_eq!(
            existing_config(Some(r#"{"permissions":{}}"#), p).unwrap(),
            json!({ "permissions": {} })
        );
    }

    #[test]
    fn registers_grok_agent() {
        // Grok 使用与现有 JSON Hook 相同的安装入口。
        assert!(find("grok").is_ok());
    }

    #[test]
    fn registers_opencode_agent() {
        // OpenCode 虽使用插件文件，也必须由统一安装入口识别。
        assert!(find("opencode").is_ok());
    }

    #[test]
    fn registers_kimi_agent() {
        // Kimi Code 使用 ~/.kimi-code/config.toml 的 [[hooks]] 数组。
        assert!(find("kimi").is_ok());
        let s = spec("kimi");
        assert_eq!(s.dir, ".kimi-code");
        assert_eq!(s.file, "config.toml");
        assert!(matches!(s.delivery, Delivery::Toml));
    }

    #[test]
    fn kimi_toml_adds_working_attention_and_finished_hooks() {
        let out = merge_kimi_hooks("", spec("kimi"));
        assert!(out.contains("event = \"UserPromptSubmit\""));
        assert!(out.contains("event = \"PermissionRequest\""));
        assert!(out.contains("event = \"Stop\""));
        #[cfg(windows)]
        {
            assert!(out.contains("terax_notify_kimi_working.cmd"));
            assert!(out.contains("terax_notify_kimi_attention.cmd"));
            assert!(out.contains("terax_notify_kimi_finished.cmd"));
            let twice = merge_kimi_hooks(&out, spec("kimi"));
            assert_eq!(
                twice.matches("terax_notify_kimi_finished.cmd").count(),
                1,
                "expected single finished hook:\n{twice}"
            );
        }
        #[cfg(not(windows))]
        {
            assert!(out.contains("__terax_notify kimi working") || out.contains("notify;Terax;kimi;working"));
            assert!(out.contains("notify;Terax;kimi;attention") || out.contains("__terax_notify kimi attention"));
            assert!(out.contains("notify;Terax;kimi;finished") || out.contains("__terax_notify kimi finished"));
            let twice = merge_kimi_hooks(&out, spec("kimi"));
            assert_eq!(
                twice.matches("notify;Terax;kimi;finished").count()
                    + twice.matches("__terax_notify kimi finished").count(),
                1,
                "expected single finished hook:\n{twice}"
            );
        }
    }

    #[test]
    fn kimi_toml_preserves_foreign_hooks() {
        let existing = r#"default_model = "kimi-code/k3"

[[hooks]]
event = "Stop"
command = "echo foreign"
timeout = 10
"#;
        let out = merge_kimi_hooks(existing, spec("kimi"));
        assert!(out.contains("echo foreign"));
        assert!(out.contains("default_model = \"kimi-code/k3\""));
        #[cfg(windows)]
        assert!(out.contains("terax_notify_kimi_finished.cmd"));
        #[cfg(not(windows))]
        assert!(
            out.contains("__terax_notify kimi finished") || out.contains("notify;Terax;kimi;finished")
        );
    }

    #[test]
    fn opencode_plugin_emits_attention_and_finished_markers() {
        let source = opencode_plugin_source();
        assert!(source.contains("Managed by Terax: agent notifications"));
        assert!(source.contains("permission.asked"));
        assert!(source.contains("opencode;attention"));
        assert!(source.contains("session.idle"));
        assert!(source.contains("opencode;finished"));
        assert!(source.contains("TERAX_TERMINAL"));
    }

    #[test]
    fn opencode_plugin_refuses_foreign_file() {
        assert!(can_write_opencode_plugin(None));
        assert!(can_write_opencode_plugin(Some(opencode_plugin_source())));
        assert!(!can_write_opencode_plugin(Some(
            "export const UserPlugin = () => ({})",
        )));
    }

    #[test]
    fn opencode_plugin_status_requires_owned_finished_hook() {
        assert!(opencode_plugin_installed(opencode_plugin_source()));
        assert!(!opencode_plugin_installed(
            "export const UserPlugin = () => ({})",
        ));
        assert!(!opencode_plugin_installed(
            "Managed by Terax: agent notifications session.idle",
        ));
    }

    #[test]
    fn grok_adds_working_and_finished_hooks() {
        let out = merge_hooks(json!({}), spec("grok"));
        assert_eq!(hook_count(&out, "UserPromptSubmit"), 1);
        assert_eq!(hook_count(&out, "Stop"), 1);
        // Grok Notification 不映射为 attention，避免回合结束误报「需要输入」。
        assert_eq!(hook_count(&out, "Notification"), 0);
        #[cfg(windows)]
        assert!(
            command(&out, "Stop", 0).contains("terax_notify_grok_finished.cmd"),
            "{}",
            command(&out, "Stop", 0)
        );
        #[cfg(not(windows))]
        assert!(command(&out, "Stop", 0).contains("__terax_notify grok finished") || command(&out, "Stop", 0).contains("notify;Terax;grok;finished"));
    }

    #[test]
    fn grok_merge_removes_legacy_notification_attention_hook() {
        let legacy = json!({
            "hooks": {
                "Notification": [{
                    "hooks": [{
                        "type": "command",
                        "command": "__terax_notify grok attention"
                    }]
                }]
            }
        });
        let out = merge_hooks(legacy, spec("grok"));
        assert_eq!(hook_count(&out, "Notification"), 0);
        assert_eq!(hook_count(&out, "Stop"), 1);
    }

    #[cfg(windows)]
    #[test]
    fn windows_osc_command_is_sh_c_safe() {
        // Windows 使用旁路 .cmd，避免 Grok 走 sh -c + GUI 假 exit 1。
        let cmd = osc_command("grok", "working");
        assert!(
            cmd.ends_with("terax_notify_grok_working.cmd"),
            "expected notify cmd path, got {cmd}"
        );
        assert!(
            !cmd.contains(' '),
            "spaces force sh -c on Grok; got {cmd}"
        );
        assert!(
            !cmd.contains('>'),
            "redirects force sh -c on Grok; got {cmd}"
        );
        let path = std::path::Path::new(&cmd);
        assert!(path.is_file(), "notify cmd should be written: {cmd}");
        let body = std::fs::read_to_string(path).unwrap();
        assert!(body.contains("__terax_notify grok working"));
        assert!(body.contains("exit /b 0"));
    }

    #[cfg(windows)]
    #[test]
    fn ancestor_pids_includes_parent() {
        // 冒烟：至少能解析到当前进程的父 PID（测试宿主）。
        let ancestors = ancestor_pids(4);
        assert!(
            !ancestors.is_empty(),
            "expected at least one ancestor pid"
        );
    }

    #[test]
    fn parse_notify_ipc_accepts_agent_event_pid() {
        assert_eq!(
            parse_notify_ipc("grok finished 4242"),
            Some(("grok", "finished", NotifyTarget::Pid(4242), Some(4242), None))
        );
        assert_eq!(
            parse_notify_ipc("grok finished pid:4242"),
            Some(("grok", "finished", NotifyTarget::Pid(4242), Some(4242), None))
        );
        assert_eq!(
            parse_notify_ipc("kimi finished pty:7"),
            Some(("kimi", "finished", NotifyTarget::Pty(7), None, None))
        );
        assert_eq!(
            parse_notify_ipc("kimi finished pty:7 pid:99"),
            Some(("kimi", "finished", NotifyTarget::Pty(7), Some(99), None))
        );
        assert!(parse_notify_ipc("grok finished").is_none());
        assert!(parse_notify_ipc("grok finished 1 extra").is_none());
    }

    #[test]
    fn parse_notify_ipc_reads_tree_verdict() {
        assert_eq!(
            parse_notify_ipc("claude finished pty:3 pid:99 tree:1"),
            Some((
                "claude",
                "finished",
                NotifyTarget::Pty(3),
                Some(99),
                Some(true)
            ))
        );
        assert_eq!(
            parse_notify_ipc("claude finished pid:99 tree:0"),
            Some((
                "claude",
                "finished",
                NotifyTarget::Pid(99),
                Some(99),
                Some(false)
            ))
        );
        assert!(parse_notify_ipc("claude finished pid:99 tree:maybe").is_none());
    }

    #[test]
    fn exe_tokens_distinguish_claude_from_grok() {
        assert!(exe_matches_agent_tokens("claude.exe", agent_exe_tokens("claude")));
        assert!(exe_matches_agent_tokens("GROK.EXE", agent_exe_tokens("grok")));
        assert!(!exe_matches_agent_tokens("grok.exe", agent_exe_tokens("claude")));
        assert!(!exe_matches_agent_tokens("claude.exe", agent_exe_tokens("grok")));
        assert!(exe_matches_agent_tokens("claude-code.exe", agent_exe_tokens("claude")));
        assert!(path_matches_agent(
            r"C:\Users\x\.grok\bin\agent.exe",
            "grok"
        ));
        assert!(!path_matches_agent(
            r"C:\Users\x\.grok\bin\agent.exe",
            "claude"
        ));
    }

    #[cfg(windows)]
    #[test]
    fn windows_status_needle_survives_json_backslash_escaping() {
        // 状态针必须能在 JSON 原文（`\\`）中命中，不能依赖未转义的整段路径。
        let out = merge_hooks(json!({}), spec("grok"));
        let raw = serde_json::to_string_pretty(&out).unwrap();
        assert!(raw.contains(&status_needle(spec("grok"), "finished")));
        assert!(windows_exe_matches(&raw));
    }

    #[cfg(windows)]
    #[test]
    fn windows_status_needle_includes_current_executable() {
        let cmd = hook_command(spec("codex"), "finished");
        let exe_dir = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        assert!(cmd.contains(&exe_dir), "cmd={cmd} dir={exe_dir}");
        assert!(status_needle(spec("codex"), "finished").contains("terax_notify_codex_finished.cmd"));
    }
}
