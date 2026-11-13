//! «Сохранить отчёт»: everything needed to say why Antigravity is or is not
//! answering, in one paste.
//!
//! «Поставил анлокер, но не помогает что-то» plus a screenshot of a green card
//! was undiagnosable (G50): the one fact that mattered - the client was not
//! going through us at all - was nowhere on screen. This puts every fact the
//! window and the relay have into plain text a user can send to the group.
//!
//! Nothing secret goes in: paths are masked the way the window shows them, the
//! built-in exits are named only as «встроенный выход» (I46), and the relay's
//! log - which is included - never names an address of ours in the first place.

use std::fmt::Write as _;

use crate::gate::{Report, View};
use crate::ops::{State, Status};
use crate::utils::mask_path;

/// The report exists in development builds only (owner, 2026-11-03: «создавать
/// отчёты больше не надо»). Release builds show no button, no TUI row and no
/// `o` key, and no message sends a user to «Сохранить отчёт»; the relay's logs
/// are untouched.
pub const ENABLED: bool = cfg!(debug_assertions);

/// The tail of a «what to try» message: the report when it can be made, the
/// group otherwise.
pub fn when_stuck() -> &'static str {
    if ENABLED {
        "请点击“保存诊断报告”，将桌面上的报告文件发送到群组反馈。"
    } else {
        "请在群组中反馈此问题。"
    }
}

/// How much of the relay's own log goes in: enough to cover the last few
/// minutes of a busy session, short enough to paste into a chat.
const LOG_LINES: usize = 60;

pub fn build(status: Option<&Status>, view: &View) -> String {
    let mut out = String::new();
    let now = crate::utils::local_clock().map_or_else(String::new, |c| c.hms());
    let _ = writeln!(
        out,
        "Antigravity 解锁器 v{} — 诊断报告 {}",
        crate::update::current_version(),
        now
    );

    match status {
        Some(s) => status_part(&mut out, s),
        None => {
            let _ = writeln!(out, "尚未读取系统状态。");
        }
    }

    let _ = writeln!(out, "\n— Antigravity（客户端日志）");
    match view.answered {
        Some(a) => {
            let _ = writeln!(
                out,
                "最近一次模型回复：{}（近 12 小时共 {} 次）",
                super::status::ago_text(a.ago),
                a.count
            );
        }
        None => {
            let _ = writeln!(out, "近 12 小时的日志中没有模型回复记录。");
        }
    }
    match view.refused_long {
        Some(r) => {
            let _ = writeln!(
                out,
                "最近一次错误 400：{}（近 10 分钟共 {} 条）",
                super::status::ago_text(r.ago),
                view.seen.map_or(0, |s| s.count)
            );
        }
        None => {
            let _ = writeln!(out, "近 12 小时的日志中没有错误 400 记录。");
        }
    }
    if let Some(o) = &view.overloaded {
        let model = if o.model.is_empty() {
            String::new()
        } else {
            format!("，模型 {}", o.model)
        };
        let _ = writeln!(
            out,
            "错误 503（Google{model} 暂无可用容量）：{}，近 10 分钟共 {} 条",
            super::status::ago_text(o.ago),
            o.count
        );
    }
    if let Some(e) = &view.server_errors {
        let _ = writeln!(
            out,
            "近 12 小时 Google 服务端错误（503/429/500）：{} 次，最近一次：{}",
            e.count,
            super::status::ago_text(e.ago)
        );
    }
    if let Some(u) = &view.unauthorized {
        let _ = writeln!(
            out,
            "错误 401（身份凭据被拒绝）：{}，近 10 分钟共 {} 条",
            super::status::ago_text(u.ago),
            u.count
        );
    }
    // When, and on which host - never the link: it carries a one-time token,
    // and the report is made to be sent to someone else.
    if let Some(v) = &view.verify {
        let host = v
            .url
            .strip_prefix("https://")
            .and_then(|r| r.split(['/', '?', '#']).next())
            .unwrap_or("");
        let _ = writeln!(
            out,
            "Google 要求验证账号：{}（{host}）",
            super::status::ago_text(v.ago)
        );
    }

    // Fresh from the file, not the window's copy: the route table's ages move
    // every pass and the watcher does not wake the window for them.
    let _ = writeln!(out, "\n— 解锁服务（运行记录）");
    match crate::gate::read() {
        Some(r) => relay_part(&mut out, &r),
        None => {
            let _ = writeln!(out, "没有运行记录，服务尚未启动或版本过旧。");
        }
    }

    let _ = writeln!(out, "\n— 后台服务日志（最近记录）");
    let log = std::fs::read_to_string(crate::dns_forwarder::log_path()).unwrap_or_default();
    let lines: Vec<&str> = log.lines().collect();
    let from = lines.len().saturating_sub(LOG_LINES);
    let own = crate::upstream::configured().map(|u| u.display());
    for line in &lines[from..] {
        let _ = writeln!(out, "{}", mask_addresses(line, own.as_deref()));
    }
    if lines.is_empty() {
        let _ = writeln!(out, "（空）");
    }
    out
}

fn onoff(s: &State) -> String {
    match s {
        State::On => "开启".to_string(),
        State::Off => "关闭".to_string(),
        State::Partial(n) => format!("部分启用（{n}）"),
        State::OffNote(n) => format!("关闭（{n}）"),
        State::Blocked(n) => format!("不可用（{n}）"),
    }
}

fn status_part(out: &mut String, s: &Status) {
    let _ = writeln!(out, "管理员权限：{}", if s.admin { "是" } else { "否" });
    let _ = writeln!(out, "\n— 安装位置");
    for row in &s.installs {
        let path = row
            .path
            .as_ref()
            .map(|p| mask_path(&p.display().to_string()))
            .unwrap_or_else(|| "未找到".to_string());
        let patched = match row.patched {
            Some(true) => "已应用补丁",
            Some(false) => "未应用补丁",
            None => "尚未检查",
        };
        let _ = writeln!(out, "{}: {} — {}", row.label, path, patched);
    }
    let _ = writeln!(out, "\n— 功能开关");
    let _ = writeln!(out, "解除登录限制：{}", onoff(&s.client_patch));
    let _ = writeln!(out, "自动补丁：{}", onoff(&s.watchdog));
    let _ = writeln!(out, "DNS 解锁：{}", onoff(&s.dns));
    let _ = writeln!(out, "本地代理：{}", onoff(&s.local_proxy));
    let key_present = if crate::settings::group_key().is_empty() {
        "否"
    } else {
        "有"
    };
    let _ = writeln!(
        out,
        "群组代理：{}，密钥：{}，设备码：{}",
        onoff(&s.group_proxy),
        key_present,
        crate::hwid::pc_code()
    );
    let _ = writeln!(out, "自定义代理：{}", onoff(&s.own_proxy));
    let _ = writeln!(
        out,
        "后台服务：{}{}；DNS 规则：{}",
        if s.relay_running {
            "运行中"
        } else {
            "未运行"
        },
        if s.relay_outdated {
            "，版本过旧"
        } else {
            ""
        },
        if s.rules { "有" } else { "无" }
    );
    if let Some(vpn) = s.vpn {
        let _ = writeln!(out, "VPN 状态（依据 Antigravity 连接）：{:?}", vpn);
    }
    if let Some(eg) = s.relay_egress {
        let _ = writeln!(out, "后台服务连接：{:?}", eg);
    }
}

fn relay_part(out: &mut String, r: &Report) {
    let _ = writeln!(
        out,
        "后台服务版本：{}（本程序要求 {}），记录更新于 {} 秒前",
        r.version,
        crate::dns_forwarder::RELAY_VERSION,
        r.age().as_secs()
    );
    let _ = writeln!(
        out,
        "Google 目标域名是否使用本地地址：{}",
        if r.loopback { "是" } else { "否" }
    );
    for b in &r.blockers {
        let _ = writeln!(out, "影响解锁的因素：{}", super::status::blocker_line(b));
    }
    if crate::proxy::port() != crate::proxy::DEFAULT_PORT {
        let _ = writeln!(
            out,
            "本地代理端口：{}（原默认端口：{}）",
            crate::proxy::port(),
            crate::proxy::DEFAULT_PORT
        );
    }
    if r.started_at != 0 {
        let _ = writeln!(
            out,
            "后台服务最近收到互联网响应：{}",
            if r.reached_at == 0 {
                "自启动以来尚未收到".to_string()
            } else {
                format!(
                    "{} 秒前",
                    crate::gate::now_unix().saturating_sub(r.reached_at)
                )
            }
        );
    }
    let _ = writeln!(
        out,
        "VPN 是否接管默认路由：{}{}",
        if r.tunnel { "是" } else { "否" },
        if r.vpn_exit.is_empty() {
            String::new()
        } else {
            format!("，出口：{}", r.vpn_exit)
        }
    );
    // The table's own first usable row, not `r.route`: that one is the
    // hysteresis memory (`routes::leader`) and can still name a route the order
    // has moved past, which put a headline in the report contradicting the
    // table printed right under it (field report, 2026-09-20).
    let first = r
        .routes
        .iter()
        .find(|row| row.usable)
        .map(|row| row.label.as_str())
        .unwrap_or(r.route.as_str());
    let _ = writeln!(out, "当前首选线路：{}", non_empty(first));
    if let Some(ok) = &r.last_ok {
        let _ = writeln!(
            out,
            "后台服务最近检测到模型回复：{} 秒前，线路：{}",
            crate::gate::now_unix().saturating_sub(ok.at),
            non_empty(&ok.route)
        );
    }
    if let Some(e) = &r.last_400 {
        let _ = writeln!(
            out,
            "后台服务最近检测到错误 400：{} 秒前，记录数：{}，线路：{}{}",
            crate::gate::now_unix().saturating_sub(e.at),
            e.count,
            non_empty(&e.route),
            if e.bypassed {
                "（未经过解锁服务）"
            } else {
                ""
            }
        );
        if !e.acted.is_empty() {
            let _ = writeln!(out, "已采取的操作：{}", e.acted);
        }
    }
    let _ = writeln!(out, "候选线路（按优先级排列）：");
    for row in &r.routes {
        let mut parts: Vec<String> = Vec::new();
        // One state word, not two: «доступен … отложен ещё на 10 мин» is what a
        // benched-but-still-offered route used to print, and it reads as a
        // contradiction to the person pasting it.
        parts.push(match (row.usable, row.bench_left) {
            (false, _) => "当前未使用".to_string(),
            (true, Some(b)) => format!("还需暂缓 {} 分钟，仍在候选列表中", b / 60 + 1),
            (true, None) => "可用".to_string(),
        });
        if let Some(ms) = row.latency_ms {
            parts.push(format!("{ms} 毫秒"));
        }
        if row.proven {
            parts.push("已通过模型回复验证".to_string());
        }
        if let Some(a) = row.ok_ago {
            parts.push(format!("{a} 秒前收到回复"));
        }
        if let Some(a) = row.refused_ago {
            parts.push(format!("{a} 秒前出现错误 400"));
        }
        // The tally, not just the two timestamps: «ответ 34 с назад, ошибка 400
        // 14 с назад» is the same line for a route that answers nine times out
        // of ten and for one that answers once in fifty, and three field
        // reports (2026-09-21) turned on telling those apart.
        if row.answers > 0 || row.refusals > 0 {
            parts.push(format!("回复 {} 次，拒绝 {} 次", row.answers, row.refusals));
        }
        if let Some(b) = row.bench_left.filter(|_| !row.usable) {
            parts.push(format!("还需暂缓 {} 分钟", b / 60 + 1));
        }
        if row.open > 0 {
            parts.push(format!("当前连接数：{}", row.open));
        }
        let _ = writeln!(out, "  {} — {}", row.label, parts.join(", "));
    }
}

/// One line of the relay's log with the user's own proxy, and any route's exit
/// address, taken out.
///
/// Since `2.15.0_1` the relay no longer writes that address at all, but a log
/// written by an older build is still in the file after the upgrade, and it did:
/// `свой прокси user:***@host:port: <why>` - their login and their server, on
/// the way into a public chat. Matching only the proxy configured *now* missed
/// every line from one they had before or had since removed, so this goes by
/// shape: any `…:***@…` credential token, and the address right after
/// «свой прокси», whatever it is. The configured value is masked too, for any
/// line that names it some other way.
fn mask_addresses(line: &str, configured: Option<&str>) -> String {
    const MASK: &str = "<自定义代理>";
    let mut out: String = line
        .split(' ')
        .map(|token| {
            if token.contains(":***@") {
                // Keep the colon that separated the address from the reason.
                if token.ends_with(':') {
                    format!("{MASK}:")
                } else {
                    MASK.to_string()
                }
            } else {
                token.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    // `свой прокси host:port: why` from before the credential form existed: an
    // address token is the one that still has a colon inside it (`host:port`).
    // Old Russian relay logs remain on upgrades and must still be redacted.
    for label in ["自定义代理 ", "свой прокси "] {
        if let Some(at) = out.find(label) {
            let rest = &out[at + label.len()..];
            let token = rest.split(' ').next().unwrap_or("");
            let bare = token.strip_suffix(':').unwrap_or(token);
            if bare.contains(':') && bare != MASK {
                let masked = if token.ends_with(':') {
                    format!("{MASK}:")
                } else {
                    MASK.to_string()
                };
                out = format!(
                    "{}{}{}",
                    &out[..at + label.len()],
                    masked,
                    &rest[token.len()..]
                );
            }
        }
    }
    // Older builds also named a route's exit address when it moved: for the
    // user's own proxy that is their server, for a built-in exit usually the
    // exit itself (I46). The country after it stays - it is what the line says.
    for phrase in [
        "出口为 ",
        "出口已更换为 ",
        "выходит через ",
        "сменил выход на ",
    ] {
        if let Some(at) = out.find(phrase) {
            let start = at + phrase.len();
            let len = out[start..].find(' ').unwrap_or(out.len() - start);
            out.replace_range(start..start + len, "<地址>");
        }
    }
    match configured {
        Some(own) if !own.is_empty() => out.replace(own, MASK),
        _ => out,
    }
}

fn non_empty(s: &str) -> &str {
    if s.is_empty() {
        "—"
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    /// What «Сохранить отчёт» writes to the file on this machine, minus the
    /// system scan the window adds. Reads, never asserts content.
    ///
    ///     cargo test prints_the_report -- --ignored --nocapture
    #[test]
    #[ignore = "reads the live relay record and log; run with --ignored"]
    fn prints_the_report() {
        let text = super::build(None, &crate::gate::View::default());
        println!("{text}");
        assert!(text.contains("报告"));
    }

    /// The line that says whether a route half-works. Two timestamps read the
    /// same for a route answering nine times in ten and one answering once in
    /// fifty, and three field reports (2026-09-21) turned on telling them
    /// apart.
    #[test]
    fn a_route_row_says_how_often_it_answered_and_how_often_it_was_refused() {
        let relay = crate::gate::Report {
            at: crate::gate::now_unix(),
            routes: vec![crate::routes::Row {
                label: "直连".into(),
                usable: true,
                latency_ms: Some(479),
                proven: true,
                ok_ago: Some(34),
                refused_ago: Some(14),
                answers: 21,
                refusals: 30,
                open: 2,
                ..Default::default()
            }],
            ..Default::default()
        };
        // `relay_part`, not `build`: the report reads the live record off the
        // disk on purpose (the table's ages move every pass and nothing wakes
        // the window for them), so a synthesised one only reaches this half.
        let mut text = String::new();
        super::relay_part(&mut text, &relay);
        assert!(
            text.contains("回复 21 次，拒绝 30 次"),
            "the tally is missing:\n{text}"
        );
        // …and the refusal is still shown, which is the half the split of
        // `bad_at`/`refused_at` must not have cost (G76).
        assert!(text.contains("14 秒前出现错误 400"), "{text}");
    }

    use super::mask_addresses as mask;

    /// A line an older relay wrote about a proxy the user has since changed
    /// keeps nothing of it: not the login, not the server.
    #[test]
    fn an_old_credential_line_is_masked_whatever_is_configured_now() {
        let line = "12:00:01 proxy        自定义代理 ivan:***@my.server.example:3128: 不可用：超时";
        for now in [None, Some("other:***@elsewhere.example:8080")] {
            let got = mask(line, now);
            assert!(
                !got.contains("ivan") && !got.contains("my.server.example"),
                "{got}"
            );
            assert!(
                got.contains("自定义代理 <自定义代理>: 不可用：超时"),
                "{got}"
            );
        }
    }

    /// The same from before credentials were masked in the display: a bare
    /// `host:port` after the label.
    #[test]
    fn an_old_bare_address_line_is_masked() {
        let got = mask(
            "12:00:01 proxy        自定义代理 10.0.0.5:1080: 代理关闭了连接",
            None,
        );
        assert_eq!(
            got,
            "12:00:01 proxy        自定义代理 <自定义代理>: 代理关闭了连接"
        );
    }

    /// Lines about the route by name stay readable: nothing there is an address.
    #[test]
    fn route_lines_are_left_alone() {
        for line in [
            "12:00:01 proxy        自定义代理 -> daily-cloudcode-pa.googleapis.com",
            "12:00:01 proxy        自定义代理: 不可用：超时",
            "12:00:01 proxy        Google 目标线路：自定义代理（378 毫秒），原为直连（325 毫秒）",
            "12:00:01 proxy        内置出口 #1 无响应：不可用：超时",
        ] {
            assert_eq!(mask(line, None), line);
        }
    }

    /// An older relay's exit-change lines keep the country, lose the address.
    #[test]
    fn an_old_exit_address_is_masked_and_the_country_kept() {
        assert_eq!(
            mask(
                "12:00:01 proxy        自定义代理 出口为 203.0.113.9 (RU) — 属于受限地区",
                None
            ),
            "12:00:01 proxy        自定义代理 出口为 <地址> (RU) — 属于受限地区"
        );
        assert_eq!(
            mask(
                "12:00:01 proxy        内置出口 #1 出口已更换为 203.0.113.9 (NL) — 恢复使用",
                None
            ),
            "12:00:01 proxy        内置出口 #1 出口已更换为 <地址> (NL) — 恢复使用"
        );
    }

    /// The proxy configured now is masked wherever it appears.
    #[test]
    fn the_configured_proxy_is_masked_anywhere() {
        let got = mask("包含 127.0.0.1:1371 的其他内容", Some("127.0.0.1:1371"));
        assert_eq!(got, "包含 <自定义代理> 的其他内容");
    }

    #[test]
    fn russian_logs_are_redacted_after_a_language_upgrade() {
        let line = "свой прокси 10.0.0.5:1080: timeout";
        assert!(!mask(line, None).contains("10.0.0.5"));
        for label in ["выходит через ", "сменил выход на "] {
            let line = format!("{label}203.0.113.9 (RU)");
            let redacted = mask(&line, None);
            assert!(!redacted.contains("203.0.113.9"));
            assert!(redacted.contains("(RU)"));
        }
    }
}
