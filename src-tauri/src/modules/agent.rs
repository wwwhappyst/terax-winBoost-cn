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
            ("Notification", "attention"),
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
];

// Substrings identifying a hook command as ours, across every form we've ever
// emitted (legacy /dev/tty Claude, current TerminalSequence, Osc, Windows
// helper). Used to prune our own groups before reinserting so installs are
// idempotent and migrate older markers.
const OWNED_MARKERS: [&str; 3] = ["notify;Terax;", "terax;notify", "__terax_notify"];

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
        Delivery::TerminalSequence => format!(
            r#"[ -n "$TERAX_TERMINAL" ] && printf '{{"terminalSequence":"\\u001b]777;notify;Terax;{event}\\u0007"}}' || true"#
        ),
        Delivery::Osc => osc_command(spec.agent, event),
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

#[cfg(windows)]
fn osc_command(agent: &str, event: &str) -> String {
    let exe = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "terax.exe".to_string());
    format!(r#""{exe}" __terax_notify {agent} {event}"#)
}

// The stable substring that proves a given (agent, event) hook is installed.
// Kept in sync with hook_command so status reflects what enable writes.
fn status_needle(spec: &AgentSpec, event: &str) -> String {
    match spec.delivery {
        Delivery::TerminalSequence => format!("notify;Terax;{event}"),
        Delivery::Osc => {
            #[cfg(unix)]
            {
                format!("notify;Terax;{};{event}", spec.agent)
            }
            #[cfg(windows)]
            {
                hook_command(spec, event)
            }
        }
        Delivery::Plugin => String::new(),
    }
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

    for (event, marker) in spec.events {
        let arr = hooks.entry(*event).or_insert_with(|| json!([]));
        if !arr.is_array() {
            *arr = json!([]);
        }
        let arr = arr.as_array_mut().unwrap();
        arr.retain(|group| !is_ours(group) && !is_empty_group(group));
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

fn existing_config(contents: Option<&str>, path: &std::path::Path) -> Result<Value, String> {
    match contents {
        Some(s) if !s.trim().is_empty() => serde_json::from_str::<Value>(s).map_err(|e| {
            format!("{} is not valid JSON ({e}); refusing to overwrite", path.display())
        }),
        _ => Ok(json!({})),
    }
}

fn settings_path(spec: &AgentSpec) -> Result<std::path::PathBuf, String> {
    Ok(dirs::home_dir()
        .ok_or_else(|| "could not resolve home dir".to_string())?
        .join(spec.dir)
        .join(spec.file))
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
    use windows_sys::Win32::System::Console::{AttachConsole, ATTACH_PARENT_PROCESS};

    if std::env::var_os("TERAX_TERMINAL").is_none() {
        return;
    }
    unsafe {
        AttachConsole(ATTACH_PARENT_PROCESS);
    }
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("CONOUT$")
    {
        let _ = f.write_all(conout_marker(agent, event).as_bytes());
    }
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
    spec.events
        .iter()
        .all(|(_, m)| content.contains(&status_needle(spec, m)))
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
        assert!(command(&out, "Notification", 0).contains("notify;Terax;attention"));
        assert!(command(&out, "Stop", 0).contains("notify;Terax;finished"));
        assert!(command(&out, "UserPromptSubmit", 0).contains("notify;Terax;working"));
        assert!(command(&out, "Stop", 0).contains("terminalSequence"));
        assert!(!command(&out, "Stop", 0).contains("/dev/tty"));
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
        assert!(command(&out, "Notification", 0).contains("terminalSequence"));
        assert!(!command(&out, "Notification", 0).contains("/dev/tty"));
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
        assert!(command(&out, "Notification", 0).contains("notify;Terax;attention"));
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
    fn grok_adds_working_attention_and_finished_hooks() {
        let out = merge_hooks(json!({}), spec("grok"));
        assert_eq!(hook_count(&out, "UserPromptSubmit"), 1);
        assert_eq!(hook_count(&out, "Notification"), 1);
        assert_eq!(hook_count(&out, "Stop"), 1);
        assert!(command(&out, "Stop", 0).contains("__terax_notify grok finished"));
    }

    #[cfg(windows)]
    #[test]
    fn windows_status_needle_includes_current_executable() {
        // 状态检查必须识别开发版、安装版或目录变化产生的失效旧路径。
        let needle = status_needle(spec("codex"), "finished");
        let exe = std::env::current_exe().unwrap().display().to_string();
        assert!(needle.contains(&exe));
    }
}
