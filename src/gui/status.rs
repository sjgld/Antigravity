//! The one line at the top of the window: is it working, and if not, what now.
//!
//! Everything below it is mechanism - which DNS answers, which route, which
//! switch - and a user who is not a programmer should not have to read any of
//! it to know whether Antigravity will answer. So the window's whole verdict is
//! derived here, as a pure function of facts the window already has, and it is
//! tested cell by cell: this is the sentence a user acts on.
//!
//! The rules that shaped it, each one a bug that shipped before:
//!
//! * **"Работает" needs proof.** Only a model answer in the client's own log
//!   turns the card green. Silence after a refusal used to read as "fixed" -
//!   and silence is also exactly what a user who gave up produces (G50).
//! * **One action at most, and a real one.** A card that says what is wrong
//!   carries the button that fixes it, or says plainly that there is nothing
//!   to press.
//! * **What was done is said by the side that did it** (the relay's record),
//!   never inferred from a switch being on (I58).

use std::time::Duration;

use crate::gate::View;
use crate::ops::{Cap, Status};

/// How the card is coloured, and how loud it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// Proven working.
    Ok,
    /// Set up, nothing proven yet: waiting for the user's next message.
    Wait,
    /// A refusal was seen and is being dealt with.
    Fixing,
    /// Something only the user can do.
    Action,
    /// Switched off, or nothing to work on.
    Off,
}

/// The one button a card may carry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Turn everything on.
    EnableAll,
    /// Restart this program elevated.
    Elevate,
    /// Reinstall and restart the service.
    Repair,
    /// Open Google's verification page (`Facts::verify`) in the browser.
    Verify,
}

impl Action {
    pub fn label(self) -> &'static str {
        match self {
            Action::EnableAll => "全部启用",
            Action::Elevate => "以管理员身份重启",
            Action::Repair => "修复",
            Action::Verify => "前往浏览器验证账号",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Headline {
    pub tone: Tone,
    pub title: String,
    pub detail: String,
    pub action: Option<Action>,
}

/// What the relay said it did about the last refusal.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Answered {
    pub acted: String,
    /// No tunnel of ours carried it: the client went to Google by itself.
    pub bypassed: bool,
}

/// Everything the verdict is made of. Plain values, so a test can build any
/// state the window can be in.
#[derive(Debug, Clone, Default)]
pub struct Facts {
    /// Windows and elevated (or not Windows, where nothing needs it).
    pub admin: bool,
    /// At least one Antigravity install was found.
    pub installs_found: bool,
    /// The account patch is on everywhere it can be.
    pub patch_on: bool,
    /// The 400 bypass is on: its DNS half, which everything else rides on - the
    /// local proxy lives in the same service, and the loopback door only opens
    /// once the relay answers the gate hosts.
    pub bypass_on: bool,
    pub relay_running: bool,
    pub relay_outdated: bool,
    /// Our DNS rules are in. Without them nothing on the machine asks the
    /// service anything, however healthy it is.
    pub rules: bool,
    /// The relay's record is fresh, i.e. it is alive and saying what it does.
    pub relay_reporting: bool,
    /// The newest refusal in a client log (over the same twelve hours as
    /// `answer`): how long ago, and how many lines in the last ten minutes.
    pub refusal: Option<(Duration, usize)>,
    /// The newest model answer in a client log, how long ago.
    pub answer: Option<Duration>,
    /// The relay's answer to that refusal, when it wrote one after it.
    pub answered: Option<Answered>,
    /// The route the relay credited with the last answer, or the one it would
    /// use now.
    pub route: Option<String>,
    /// What keeps the local proxy from listening, as the relay diagnosed it.
    pub proxy_blocked: Option<crate::gate::Blocker>,
    /// What keeps the gate hosts' door on `:443` shut, the same way.
    pub door_blocked: Option<crate::gate::Blocker>,
    /// What keeps the DNS relay off `127.0.0.53:53`. Unlike the other two this
    /// listener cannot be moved - the NRPT rules name an address, not a port -
    /// so naming the holder is the whole of what can be done about it.
    pub dns_blocked: Option<crate::gate::Blocker>,
    /// The relay has run for minutes and nothing on the internet answered it.
    pub cut_off: bool,
    /// Whether this window itself reaches the internet - asked only while the
    /// relay is cut off, and what tells "no internet" from "something blocks
    /// the relay alone".
    pub net_ok: Option<bool>,
    /// The relay's own exe: the file an antivirus exception has to name.
    pub relay_exe: String,
    /// The newest time Google asked for the account to be verified: how long
    /// ago, and the page it gave. `headline` decides whether it still stands.
    pub verify: Option<(Duration, String)>,
    /// The newest 503 "no capacity" in the last ten minutes: how long ago, and
    /// the model it named (empty if none).
    pub overloaded: Option<(Duration, String)>,
    /// The newest 401 in the last ten minutes: how long ago, and how many.
    pub unauthorized: Option<(Duration, usize)>,
}

/// One 401 can be a token refreshed a moment late; a credential Google stopped
/// accepting fails every call, hundreds of times (G11).
const UNAUTH_LINES: usize = 2;

/// How long a verification demand stays on the card with no answer after it.
/// The page carries a one-time token; past this the client has usually asked
/// again and logged a fresh one, and an old link is more likely to fail than
/// help.
const VERIFY_FOR: Duration = Duration::from_secs(30 * 60);

/// About three refused turns (each writes four lines) with no answer in
/// between: past that, "send it again" has been tried and did not help.
const STUCK_LINES: usize = 12;

/// How long a refusal is "happening now" and the card says «Чиним». After it,
/// with no answer since, the card asks for a check instead: nothing is being
/// fixed any more, and nothing has been shown to work either.
const FIXING_FOR: Duration = Duration::from_secs(10 * 60);

/// How far *before* a refusal the relay's note may be stamped and still be an
/// answer to it: the whole-second quantisation both stamps carry, and nothing
/// more. A note stamped earlier is an answer to something else (I58).
const EPISODE_SLACK: u64 = 3;

/// Whether the relay's note answers a refusal stamped at `refusal_at`.
fn answers(episode_at: u64, refusal_at: u64) -> bool {
    episode_at.saturating_add(EPISODE_SLACK) >= refusal_at
}

impl Facts {
    /// Read off a status snapshot and the gate watcher's last view, which is
    /// `gate_age` old: the watcher sends an age measured at its own tick and
    /// then stays quiet while nothing changes, so the front end ages its copy.
    /// Both front ends (window and terminal) call this, so they cannot answer
    /// differently.
    pub fn read(s: &Status, gate: &View, gate_age: Duration) -> Facts {
        let aged = |ago: Duration| ago + gate_age;
        // Newest over twelve hours, so an old refusal still outranks an older
        // answer; counted over the last ten minutes, which is what "it keeps
        // happening" means.
        let refusal = gate
            .refused_long
            .map(|x| (aged(x.ago), gate.seen.map_or(0, |s| s.count)));
        let answer = gate.answered.map(|x| aged(x.ago));
        // The relay's record is only worth anything while the relay runs: a record
        // outlives its writer by up to `STALE_AFTER`, and "обход перехватил" about a
        // dead service is the worst sentence this card could say (I58).
        let relay = gate.relay.as_ref().filter(|_| s.relay_running);
        let answered = refusal.and_then(|(ago, _)| {
            if ago > crate::gate::RECENT {
                return None;
            }
            let refusal_at = crate::gate::now_unix().saturating_sub(ago.as_secs());
            relay
                .and_then(|r| r.last_400.as_ref())
                .filter(|e| answers(e.at, refusal_at))
                .map(|e| Answered {
                    acted: e.acted.clone(),
                    bypassed: e.bypassed,
                })
        });
        // The path of the last answer when the relay saw one recently - and then
        // exactly that, empty included: an answer no tunnel of ours carried went
        // around us, and naming the route we would have used instead would be a
        // claim about traffic that never touched it. Otherwise the route in force.
        let route = relay.and_then(|r| {
            let recent_ok = r.last_ok.as_ref().filter(|ok| {
                crate::gate::now_unix().saturating_sub(ok.at)
                    <= crate::gate::ANSWER_RECENT.as_secs()
            });
            match recent_ok {
                Some(ok) => (!ok.route.is_empty()).then(|| ok.route.clone()),
                None => (!r.route.is_empty()).then(|| r.route.clone()),
            }
        });
        Facts {
            admin: s.admin || !cfg!(target_os = "windows"),
            installs_found: s.installs.iter().any(|r| r.path.is_some()),
            patch_on: s.client_patch.is_on(),
            bypass_on: s.bypass_on(),
            relay_running: s.relay_running,
            relay_outdated: s.relay_outdated,
            // Rules are only owed while the DNS switch is on: a route running alone
            // (own proxy, group, exits) has none by design.
            rules: s.rules || !s.dns.is_on() || !cfg!(target_os = "windows"),
            relay_reporting: relay.is_some(),
            refusal,
            answer,
            answered,
            route,
            proxy_blocked: blocker(relay, "proxy"),
            door_blocked: blocker(relay, "door"),
            dns_blocked: blocker(relay, "dns"),
            cut_off: relay.is_some_and(crate::gate::Report::cut_off),
            net_ok: gate.net_ok,
            relay_exe: relay.map(|r| r.exe.clone()).unwrap_or_default(),
            verify: gate.verify.as_ref().map(|v| (aged(v.ago), v.url.clone())),
            overloaded: gate
                .overloaded
                .as_ref()
                .map(|o| (aged(o.ago), o.model.clone())),
            unauthorized: gate.unauthorized.map(|u| (aged(u.ago), u.count)),
        }
    }
}

fn blocker(relay: Option<&crate::gate::Report>, what: &str) -> Option<crate::gate::Blocker> {
    relay?.blockers.iter().find(|b| b.what == what).cloned()
}

/// A provider's name as it is shown: an acronym stays one, anything else gets
/// one capital.
pub fn provider_name(name: &str) -> String {
    let name = crate::resolvers::display_provider_name(name);
    // The two the pool carries are written the way their owners write them.
    match name {
        "dns-ai.ru" => return "DNS-AI.ru".to_string(),
        "eu.geohide.ru" => return "eu.GeoHide.ru".to_string(),
        _ => {}
    }
    let lead: String = name.chars().take_while(|c| c.is_alphabetic()).collect();
    if lead.eq_ignore_ascii_case("dns") {
        return name.to_uppercase();
    }
    let mut chars = name.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// The master switch of the bypass: what it is called and what it does.
pub const BYPASS_TEXT: (&str, &str) = (
    "解除 AI 聊天的地区限制（错误 400）",
    "自动寻找可用的 Google 连接线路；当前线路失效时自动切换，兼容有无 VPN 的环境。",
);

/// What a switch is called and what it does, in both front ends.
pub fn switch_text(cap: Cap) -> (&'static str, &'static str) {
    match cap {
        Cap::ClientPatch => (
            "解除账号登录限制",
            "解除受限地区 Google 账号的客户端登录限制。",
        ),
        Cap::Watchdog => (
            "自动补丁",
            "检测到 Antigravity 安装或更新后，自动重新应用补丁。",
        ),
        Cap::Dns => (
            "DNS 解锁",
            "由本机后台服务接管两个 Google 域名的解析，通过解锁服务提供连接地址。DNS 解锁需要此服务运行。",
        ),
        Cap::LocalProxy => (
            "本地代理",
            "Antigravity 通过本机代理连接这两个 Google 服务。代理自动选择可用线路，只转发数据，不解密通信内容。",
        ),
        Cap::GroupProxy => (
            "群组代理",
            "群组提供的专用服务器，仅转发模型请求。",
        ),
        Cap::OwnProxy => (
            "自定义 HTTP 代理",
            "使用你在支持地区的 HTTP 代理，程序会优先尝试。格式：用户名:密码@地址:端口；无需认证时填写 地址:端口，例如 ivan:secret@203.0.113.9:3128。不支持 SOCKS。Google 仍可能拒绝数据中心出口。",
        ),
        Cap::DnsRotation => (
            "DNS 服务器轮换",
            "开启时并行查询所有已启用的服务器，并与参考解析结果核对。关闭时仅使用列表中第一个已启用的服务器，无备用服务器。",
        ),
    }
}

/// The card for a listener the relay could not bind, by what stopped it.
///
/// Every text ends in something the user can do; the relay retries each minute,
/// so the fixes that need no restart say so.
fn blocked(b: &crate::gate::Blocker, exe: &str) -> Headline {
    let port = b.addr.rsplit(':').next().unwrap_or(&b.addr);
    let effect = match b.what.as_str() {
        "door" => "Antigravity 未经过解锁服务连接 Google，因此收到错误 400",
        "dns" => "地址替换未生效，Google 请求速度降低",
        _ => "地区限制解锁未完全生效",
    };
    // Port 53 held by a service is the one case where the image name is worse
    // than useless: it is `svchost.exe`, which the user cannot act on. What
    // actually holds it is nearly always Internet Connection Sharing, started
    // behind the user's back by the Hyper-V default switch, the mobile hotspot
    // or a virtual machine - so that is what the card says instead.
    let system_holder = b.by.is_empty() || b.by.eq_ignore_ascii_case("svchost.exe");
    if b.what == "dns" && b.cause == "held" && system_holder {
        return Headline {
            tone: Tone::Action,
            title: format!("端口 {port} 被系统服务占用"),
            detail: format!(
                "Windows 系统服务占用了端口 {port}，导致{effect}。常见原因是 Hyper-V、移动热点或虚拟机启用的“Internet 连接共享（ICS）”。可在“服务”中停止 ICS 并将启动类型设为“禁用”；解锁服务将在一分钟内重新尝试监听。"
            ),
            action: Some(Action::Repair),
        };
    }
    let (title, detail) = match b.cause.as_str() {
        "held" => {
            let who = if b.by.is_empty() {
                "其他程序".to_string()
            } else {
                format!("程序“{}”", b.by)
            };
            (
                format!("端口 {port} 被其他程序占用"),
                format!(
                    "{who} 占用了本机端口 {port}，导致{effect}。关闭该程序或其 Web 服务后，解锁服务将在一分钟内重新尝试；也可点击“修复”。"
                ),
            )
        }
        "reserved" => (
            format!("端口 {port} 被 Windows 保留"),
            format!(
                "端口 {port} 被系统保留，常见于 Hyper-V、WSL 或 Docker，导致{effect}。以管理员身份打开命令提示符，依次运行 net stop winnat 和 net start winnat，然后点击“修复”。"
            ),
        ),
        "denied" => (
            "安全软件阻止了解锁服务".to_string(),
            format!(
                "安全软件或防火墙阻止服务监听端口 {port}，导致{effect}。将 {} 加入例外后，服务将在一分钟内重试，也可点击“修复”。",
                exe_or_default(exe)
            ),
        ),
        _ => (
            format!("端口 {port} 不可用"),
            format!(
                "解锁服务无法监听端口 {port}（{}），导致{effect}。请点击“修复”；若仍未解决，{}",
                b.error,
                super::report::when_stuck()
            ),
        ),
    };
    Headline {
        tone: Tone::Action,
        title,
        detail,
        action: Some(Action::Repair),
    }
}

/// The relay's exe as it reported it, or where it is installed by default.
fn exe_or_default(exe: &str) -> String {
    if !exe.is_empty() {
        exe.to_string()
    } else if cfg!(target_os = "windows") {
        r"C:\ProgramData\AGUnlocker\ag_dns.exe".to_string()
    } else {
        "~/.local/share/agunlocker/ag_proxy".to_string()
    }
}

/// One blocker as a line of the report: what, where, why.
pub fn blocker_line(b: &crate::gate::Blocker) -> String {
    let what = match b.what.as_str() {
        "door" => "Google 目标域名的本地地址",
        "dns" => "DNS 转发服务",
        _ => "本地代理",
    };
    let why = match b.cause.as_str() {
        "held" if !b.by.is_empty() => format!("端口被程序“{}”占用", b.by),
        "held" => "端口被其他程序占用".to_string(),
        "reserved" => "端口被 Windows 保留（Hyper-V/WSL/Docker）".to_string(),
        "denied" => "访问被拒绝，可能是安全软件或防火墙限制".to_string(),
        _ => "原因尚未确定".to_string(),
    };
    format!("{what} ({}): {why} — {}", b.addr, b.error)
}

pub fn headline(f: &Facts) -> Headline {
    // Nothing to work on. Said first: every other card assumes Antigravity is
    // installed, and a button that patches nothing is not an action.
    if !f.installs_found && !f.patch_on && !f.bypass_on {
        return Headline {
            tone: Tone::Off,
            title: "未找到 Antigravity".into(),
            detail: "请先安装 Antigravity，或点击下方对应项目旁的铅笔图标，指定安装文件夹。".into(),
            action: None,
        };
    }

    // Google itself stopped the account until it is verified. Above everything
    // the bypass says: no route and no switch changes this, only the user in a
    // browser - and the client does not always offer the button for it.
    // Only while nothing has answered since: an answer after it means the
    // account was verified, whatever the log still says.
    let verify_due = f
        .verify
        .as_ref()
        .filter(|(ago, _)| *ago <= VERIFY_FOR && f.answer.is_none_or(|a| a > *ago));
    if let Some((ago, _)) = verify_due {
        return Headline {
            tone: Tone::Action,
            title: "需要验证 Google 账号".into(),
            detail: format!(
                "Google 要求验证账号（{}）。Antigravity 可能未显示验证入口；点击下方按钮将在默认浏览器中打开 Google 验证页面，请使用同一账号。验证后回到 Antigravity 重新发送消息。",
                ago_text(*ago)
            ),
            action: Some(Action::Verify),
        };
    }

    // Google errors the bypass cannot fix and must not be blamed for: both mean
    // the request got past the gate. Only while nothing has answered since.
    let unanswered = |ago: Duration| f.answer.is_none_or(|a| a > ago);
    let unauthorized = f
        .unauthorized
        .filter(|(ago, n)| *ago <= crate::gate::RECENT && *n >= UNAUTH_LINES && unanswered(*ago));
    if let Some((ago, _)) = unauthorized {
        return Headline {
            tone: Tone::Action,
            title: "需要重新登录 Antigravity".into(),
            detail: format!(
                "Google 返回错误 401（{}），Antigravity 的登录凭据已失效。请退出账号后重新登录，解锁功能无法修复此错误。",
                ago_text(ago)
            ),
            action: None,
        };
    }
    // A refusal newer than the 503 is the news; the 503 was an earlier turn.
    let overloaded = f.overloaded.as_ref().filter(|(ago, _)| {
        *ago <= crate::gate::RECENT && unanswered(*ago) && f.refusal.is_none_or(|(r, _)| r > *ago)
    });
    if let Some((ago, model)) = overloaded {
        let which = if model.is_empty() {
            String::new()
        } else {
            format!("（模型：{model}）")
        };
        return Headline {
            tone: Tone::Action,
            title: "Google 服务器繁忙".into(),
            detail: format!(
                "Google 返回错误 503（{}），当前没有可用容量{which}。请求已到达 Google，说明连接已建立。请在 Antigravity 中换一个模型或稍后重试。",
                ago_text(*ago)
            ),
            action: None,
        };
    }

    // The bypass needs an administrator to install and to repair. Asked for
    // only when there is something it would install or repair.
    let service_broken = f.bypass_on && (!f.relay_running || f.relay_outdated || !f.rules);
    if !f.admin && (!f.bypass_on || service_broken) {
        return Headline {
            tone: Tone::Action,
            title: "需要管理员权限".into(),
            detail: "安装或修复错误 400 的解锁服务需要管理员权限；客户端登录补丁不受此限制。"
                .into(),
            action: Some(Action::Elevate),
        };
    }

    if !f.patch_on || !f.bypass_on {
        let what = match (f.patch_on, f.bypass_on) {
            (false, false) => "登录补丁和地区限制解锁均未启用。",
            (false, true) => "登录补丁未启用。",
            _ => "地区限制解锁未启用。",
        };
        let closes = if f.patch_on {
            ""
        } else {
            " Antigravity 将暂时关闭几秒。"
        };
        return Headline {
            tone: Tone::Off,
            title: "尚未全部启用".into(),
            detail: format!("{what}{closes}"),
            action: Some(Action::EnableAll),
        };
    }

    if service_broken {
        return Headline {
            tone: Tone::Action,
            title: if !f.relay_running {
                "解锁服务未运行".into()
            } else if f.relay_outdated {
                "解锁服务版本过旧".into()
            } else {
                "解锁服务未完全正常运行".into()
            },
            detail: "解决错误 400 需要后台服务。点击下方按钮将重新安装并启动服务。".into(),
            action: Some(Action::Repair),
        };
    }

    // Something on this machine keeps part of the relay down (P53). Said with
    // the thing to do about it - the 400 it causes says nothing of the sort.
    if let Some(b) = &f.proxy_blocked {
        return blocked(b, &f.relay_exe);
    }
    // The DNS listener is the other half the relay cannot open by itself. Below
    // the proxy because the proxy carries everything while it is up, and above
    // "cut off" because a dead `:53` is a named, fixable cause and being cut
    // off is a guess.
    if let Some(b) = &f.dns_blocked {
        return blocked(b, &f.relay_exe);
    }
    if f.cut_off {
        match f.net_ok {
            Some(true) => {
                return Headline {
                    tone: Tone::Action,
                    title: "安全软件或防火墙阻止了解锁服务连接".into(),
                    detail: format!(
                        "其他程序能联网，但解锁服务已连续数分钟无法连接任何服务器，可能受到安全软件或防火墙限制。可将 {} 加入安全软件例外及防火墙允许列表，然后点击“修复”。",
                        exe_or_default(&f.relay_exe)
                    ),
                    action: Some(Action::Repair),
                }
            }
            Some(false) => {
                return Headline {
                    tone: Tone::Off,
                    title: "无法连接互联网".into(),
                    detail: "解锁器和后台服务均无法连接互联网。请检查网络；若其他程序能联网，请检查安全软件或防火墙是否限制了这两个程序。"
                        .into(),
                    action: None,
                }
            }
            None => {}
        }
    }

    // A refusal newer than the newest answer is the live problem.
    let refused_last = match (f.refusal, f.answer) {
        (Some((r, _)), Some(a)) => r < a,
        (Some(_), None) => true,
        _ => false,
    };
    if refused_last {
        // With the door shut, Antigravity's calls go around the bypass: that
        // is the cause, and «Чиним» would promise a fix the relay cannot make.
        if let Some(b) = &f.door_blocked {
            return blocked(b, &f.relay_exe);
        }
        let (ago, lines) = f.refusal.unwrap_or_default();
        if ago > FIXING_FOR {
            return Headline {
                tone: Tone::Wait,
                title: "等待验证连接".into(),
                detail: format!(
                    "最近一次错误 400 出现在 {}，此后尚未收到模型回复。请在 Antigravity 中发送一条消息；如果错误再次出现，程序会自动尝试其他线路。",
                    ago_text(ago)
                ),
                action: None,
            };
        }
        if lines >= STUCK_LINES {
            return Headline {
                tone: Tone::Action,
                title: "持续出现错误 400".into(),
                detail: format!(
                    "最近一次发生在 {}。程序正在尝试候选线路，但尚未找到可用线路。如已启用 VPN 或代理，可尝试切换节点后重新发送消息；若仍未解决，{}",
                    ago_text(ago),
                    super::report::when_stuck()
                ),
                action: None,
            };
        }
        let detail = match &f.answered {
            Some(a) if a.bypassed => format!(
                "错误 400（{}）：Antigravity 未经过解锁服务连接 Google。服务将在一分钟内调整线路，请重新发送消息；若仍失败，请重启 Antigravity。",
                ago_text(ago)
            ),
            Some(a) if !a.acted.is_empty() => format!(
                "错误 400（{}）。{}。请重新发送消息。",
                ago_text(ago),
                capitalise(&a.acted)
            ),
            _ => format!(
                "错误 400（{}）。解锁服务已检测到问题，正在调整线路，请重新发送消息。",
                ago_text(ago)
            ),
        };
        return Headline {
            tone: Tone::Fixing,
            title: "正在修复".into(),
            detail,
            action: None,
        };
    }

    if let Some(ago) = f.answer {
        let route = f
            .route
            .as_deref()
            .filter(|r| !r.is_empty())
            .map(|r| format!(" · 线路：{r}"))
            .unwrap_or_default();
        return Headline {
            tone: Tone::Ok,
            title: "连接正常".into(),
            detail: format!("模型已回复：{}{route}。", ago_text(ago)),
            action: None,
        };
    }

    if !f.relay_reporting {
        return Headline {
            tone: Tone::Wait,
            title: "后台服务正在启动".into(),
            detail: "启动最多需要一分钟，完成后请在 Antigravity 中发送一条消息。".into(),
            action: None,
        };
    }

    Headline {
        tone: Tone::Wait,
        title: "已全部启用".into(),
        detail: "请在 Antigravity 聊天中发送一条消息。收到模型回复后，这里会显示验证结果。".into(),
        action: None,
    }
}

fn capitalise(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// «только что», «22 секунды назад», «3 минуты назад», «2 часа назад».
pub fn ago_text(d: Duration) -> String {
    let secs = d.as_secs();
    if secs < 15 {
        return "刚刚".to_string();
    }
    if secs < 60 {
        return format!("{} {}前", secs, plural(secs, "秒", "秒", "秒"));
    }
    let mins = secs / 60;
    if mins < 60 {
        return format!("{} {}前", mins, plural(mins, "分钟", "分钟", "分钟"));
    }
    let hours = mins / 60;
    format!("{} {}前", hours, plural(hours, "小时", "小时", "小时"))
}

/// Russian counts in three forms.
pub fn plural(n: u64, one: &'static str, few: &'static str, many: &'static str) -> &'static str {
    if n % 100 / 10 == 1 {
        return many;
    }
    match n % 10 {
        1 => one,
        2..=4 => few,
        _ => many,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn working() -> Facts {
        Facts {
            admin: true,
            installs_found: true,
            patch_on: true,
            bypass_on: true,
            relay_running: true,
            relay_outdated: false,
            rules: true,
            relay_reporting: true,
            ..Facts::default()
        }
    }

    fn secs(n: u64) -> Duration {
        Duration::from_secs(n)
    }

    /// G50, the report that started this: a refusal, then quiet. Quiet is not
    /// proof - the card must not turn green without a model answer after it.
    #[test]
    fn silence_after_a_refusal_is_not_working() {
        let f = Facts {
            refusal: Some((secs(300), 4)),
            answered: Some(Answered {
                acted: "正在重新选择 Google 服务器地址".into(),
                bypassed: false,
            }),
            ..working()
        };
        let h = headline(&f);
        assert_eq!(h.tone, Tone::Fixing);
        assert!(h.detail.contains("请重新发送消息"), "{}", h.detail);
    }

    #[test]
    fn an_answer_after_the_refusal_is_working_and_says_by_which_path() {
        let f = Facts {
            refusal: Some((secs(300), 4)),
            answer: Some(secs(60)),
            route: Some("内置出口".into()),
            ..working()
        };
        let h = headline(&f);
        assert_eq!(h.tone, Tone::Ok);
        assert_eq!(h.title, "连接正常");
        assert!(h.detail.contains("1 分钟前"), "{}", h.detail);
        assert!(h.detail.contains("内置出口"), "{}", h.detail);
    }

    /// Found live: an answer, then a refusal, then quiet. Once the refusal is
    /// older than the fixing window the card must not fall back to the answer
    /// before it and say «Работает».
    #[test]
    fn an_old_refusal_after_the_last_answer_asks_for_a_check_not_a_green_card() {
        let f = Facts {
            refusal: Some((secs(13 * 60), 0)),
            answer: Some(secs(14 * 60)),
            ..working()
        };
        let h = headline(&f);
        assert_eq!(h.tone, Tone::Wait);
        assert_eq!(h.title, "等待验证连接");
        // And an answer after that same refusal is working again.
        let f = Facts {
            refusal: Some((secs(13 * 60), 0)),
            answer: Some(secs(60)),
            ..working()
        };
        assert_eq!(headline(&f).tone, Tone::Ok);
    }

    #[test]
    fn a_refusal_after_the_last_answer_is_the_live_problem() {
        let f = Facts {
            refusal: Some((secs(20), 4)),
            answer: Some(secs(600)),
            ..working()
        };
        assert_eq!(headline(&f).tone, Tone::Fixing);
    }

    #[test]
    fn set_up_and_unproven_asks_for_a_message_and_claims_nothing() {
        let h = headline(&working());
        assert_eq!(h.tone, Tone::Wait);
        assert!(!h.detail.contains("连接正常"), "{}", h.detail);
        assert_eq!(h.action, None);
    }

    #[test]
    fn a_client_that_went_around_us_is_told_so_and_what_to_do() {
        let f = Facts {
            refusal: Some((secs(10), 4)),
            answered: Some(Answered {
                acted: String::new(),
                bypassed: true,
            }),
            ..working()
        };
        let h = headline(&f);
        assert_eq!(h.tone, Tone::Fixing);
        assert!(h.detail.contains("未经过解锁服务"), "{}", h.detail);
        assert!(h.detail.contains("重启 Antigravity"), "{}", h.detail);
    }

    #[test]
    fn many_refusals_with_no_answer_stop_promising_and_say_what_to_try() {
        let f = Facts {
            refusal: Some((secs(10), STUCK_LINES)),
            ..working()
        };
        let h = headline(&f);
        assert_eq!(h.tone, Tone::Action);
        assert_eq!(
            h.detail.contains("保存诊断报告"),
            super::super::report::ENABLED,
            "{}",
            h.detail
        );
    }

    #[test]
    fn switched_off_offers_the_one_button_and_warns_when_it_closes_antigravity() {
        let f = Facts {
            patch_on: false,
            bypass_on: false,
            ..working()
        };
        let h = headline(&f);
        assert_eq!(h.action, Some(Action::EnableAll));
        assert!(h.detail.contains("暂时关闭"), "{}", h.detail);
        // Only the bypass off: nothing gets closed.
        let f = Facts {
            bypass_on: false,
            ..working()
        };
        let h = headline(&f);
        assert_eq!(h.action, Some(Action::EnableAll));
        assert!(!h.detail.contains("暂时关闭"), "{}", h.detail);
    }

    #[test]
    fn without_admin_the_button_is_elevation_but_only_when_there_is_work_for_it() {
        let f = Facts {
            admin: false,
            bypass_on: false,
            ..working()
        };
        assert_eq!(headline(&f).action, Some(Action::Elevate));
        // Everything is installed and running: an unelevated window has nothing
        // to ask for.
        let f = Facts {
            admin: false,
            answer: Some(secs(30)),
            ..working()
        };
        assert_eq!(headline(&f).tone, Tone::Ok);
    }

    #[test]
    fn a_dead_or_old_service_is_repaired_by_one_button() {
        let f = Facts {
            relay_running: false,
            ..working()
        };
        let h = headline(&f);
        assert_eq!((h.tone, h.action), (Tone::Action, Some(Action::Repair)));
        let f = Facts {
            relay_outdated: true,
            ..working()
        };
        assert_eq!(headline(&f).title, "解锁服务版本过旧");
        // Running, current, and still useless: nothing asks it anything.
        let f = Facts {
            rules: false,
            ..working()
        };
        let h = headline(&f);
        assert_eq!(h.action, Some(Action::Repair));
        assert_eq!(h.title, "解锁服务未完全正常运行");
    }

    #[test]
    fn nothing_installed_is_said_without_a_button() {
        let f = Facts {
            installs_found: false,
            patch_on: false,
            bypass_on: false,
            ..working()
        };
        let h = headline(&f);
        assert_eq!(h.tone, Tone::Off);
        assert_eq!(h.action, None);
    }

    #[test]
    fn russian_counts_in_three_forms() {
        let minutes = |n| plural(n, "分钟", "分钟", "分钟");
        assert_eq!(minutes(1), "分钟");
        assert_eq!(minutes(2), "分钟");
        assert_eq!(minutes(5), "分钟");
        assert_eq!(minutes(11), "分钟");
        assert_eq!(minutes(21), "分钟");
        assert_eq!(minutes(22), "分钟");
        assert_eq!(minutes(0), "分钟");
    }

    #[test]
    fn an_age_reads_as_a_person_would_say_it() {
        assert_eq!(ago_text(secs(3)), "刚刚");
        assert_eq!(ago_text(secs(22)), "22 秒前");
        assert_eq!(ago_text(secs(61)), "1 分钟前");
        assert_eq!(ago_text(secs(9 * 60)), "9 分钟前");
        assert_eq!(ago_text(secs(2 * 3600 + 5)), "2 小时前");
        assert_eq!(ago_text(secs(5 * 3600)), "5 小时前");
    }

    #[test]
    fn an_acronym_stays_an_acronym_and_everything_else_gets_one_capital() {
        assert_eq!(provider_name("dns-ai.ru"), "DNS-AI.ru");
        assert_eq!(provider_name("eu.geohide.ru"), "eu.GeoHide.ru");
        assert_eq!(provider_name("自定义 DNS"), "自定义 DNS");
        assert_eq!(provider_name(crate::resolvers::CUSTOM_NAME), "自定义 DNS");
        assert_eq!(provider_name("dns.example"), "DNS.EXAMPLE");
        // Must not panic on a name the pool could grow later.
        assert_eq!(provider_name(""), "");
        assert_eq!(provider_name("1.1.1.1"), "1.1.1.1");
    }

    #[test]
    fn only_a_note_written_after_the_refusal_answers_it() {
        assert!(answers(1_000, 1_000));
        assert!(answers(1_015, 1_000));
        assert!(answers(998, 1_000));
        assert!(!answers(940, 1_000));
        assert!(!answers(0, 1_000));
        assert!(answers(u64::MAX, 1_000));
        assert!(!answers(0, u64::MAX));
    }

    fn blocker(what: &str, cause: &str, by: &str) -> crate::gate::Blocker {
        crate::gate::Blocker {
            what: what.into(),
            addr: if what == "door" {
                "127.65.71.1:443"
            } else {
                "127.0.0.1:53129"
            }
            .into(),
            cause: cause.into(),
            by: by.into(),
            error: "os error 10013".into(),
        }
    }

    /// P53: a proxy the relay cannot bind is said with the fix, even while the
    /// model still answers through what is left of the bypass.
    #[test]
    fn a_blocked_proxy_says_what_to_add_to_the_antivirus() {
        let f = Facts {
            answer: Some(secs(5)),
            proxy_blocked: Some(blocker("proxy", "denied", "")),
            relay_exe: r"C:\ProgramData\AGUnlocker\ag_dns.exe".into(),
            ..working()
        };
        let h = headline(&f);
        assert_eq!(h.tone, Tone::Action);
        assert_eq!(h.title, "安全软件阻止了解锁服务");
        assert!(
            h.detail.contains(r"C:\ProgramData\AGUnlocker\ag_dns.exe"),
            "{}",
            h.detail
        );
        assert!(h.detail.contains("53129"), "{}", h.detail);
        assert_eq!(h.action, Some(Action::Repair));
    }

    /// The door matters when the 400 is happening: then it is the reason, named
    /// with the program holding the port. While answers come, it is not news.
    #[test]
    fn a_shut_door_is_the_reason_for_a_refusal_and_names_who_holds_443() {
        let door = Some(blocker("door", "held", "vmware-hostd.exe"));
        let refused = Facts {
            refusal: Some((secs(20), 4)),
            door_blocked: door.clone(),
            ..working()
        };
        let h = headline(&refused);
        assert_eq!(h.tone, Tone::Action);
        assert_eq!(h.title, "端口 443 被其他程序占用");
        assert!(h.detail.contains("“vmware-hostd.exe”"), "{}", h.detail);
        let answering = Facts {
            answer: Some(secs(5)),
            refusal: Some((secs(60), 4)),
            door_blocked: door,
            ..working()
        };
        assert_eq!(headline(&answering).tone, Tone::Ok);
    }

    #[test]
    fn a_reserved_port_asks_for_winnat_to_be_restarted() {
        let f = Facts {
            refusal: Some((secs(20), 4)),
            door_blocked: Some(blocker("door", "reserved", "")),
            ..working()
        };
        let h = headline(&f);
        assert_eq!(h.title, "端口 443 被 Windows 保留");
        assert!(h.detail.contains("net stop winnat"), "{}", h.detail);
    }

    /// A relay nothing answers: blocked if this window gets out, offline if it
    /// does not, and no verdict before the window has asked.
    #[test]
    fn a_cut_off_relay_is_told_apart_from_a_machine_with_no_internet() {
        let blocked = Facts {
            cut_off: true,
            net_ok: Some(true),
            ..working()
        };
        let h = headline(&blocked);
        assert_eq!(h.title, "安全软件或防火墙阻止了解锁服务连接");
        assert_eq!(h.action, Some(Action::Repair));
        let offline = Facts {
            cut_off: true,
            net_ok: Some(false),
            ..working()
        };
        assert_eq!(headline(&offline).title, "无法连接互联网");
        let unasked = Facts {
            cut_off: true,
            net_ok: None,
            ..working()
        };
        assert_eq!(headline(&unasked).title, "已全部启用");
    }

    /// Google stopped the account: the card says so above everything, red, with
    /// the one button that helps - even with the bypass switched off, which has
    /// nothing to do with it.
    #[test]
    fn a_verification_demand_tops_the_card_until_an_answer_follows() {
        let url = "https://accounts.google.com/signin/continue?a=1&b=2".to_string();
        let asked = Facts {
            verify: Some((secs(120), url.clone())),
            answer: Some(secs(900)),
            bypass_on: false,
            ..working()
        };
        let h = headline(&asked);
        assert_eq!(h.tone, Tone::Action);
        assert_eq!(h.action, Some(Action::Verify));

        let answered_since = Facts {
            verify: Some((secs(120), url.clone())),
            answer: Some(secs(30)),
            ..working()
        };
        assert_ne!(headline(&answered_since).action, Some(Action::Verify));

        let stale = Facts {
            verify: Some((VERIFY_FOR + secs(1), url)),
            ..working()
        };
        assert_ne!(headline(&stale).action, Some(Action::Verify));
    }

    /// A 503 is Google out of capacity, not the bypass: the card says to pick
    /// another model, and lets go once a model answers or a 400 comes after it.
    #[test]
    fn a_503_asks_for_another_model_until_an_answer_follows() {
        let busy = Facts {
            overloaded: Some((secs(60), "gemini-3.8-flash-high".into())),
            answer: Some(secs(900)),
            ..working()
        };
        let h = headline(&busy);
        assert_eq!(h.title, "Google 服务器繁忙");
        assert!(h.detail.contains("gemini-3.8-flash-high"), "{}", h.detail);
        assert!(h.detail.contains("换一个"), "{}", h.detail);

        let answered = Facts {
            overloaded: Some((secs(60), String::new())),
            answer: Some(secs(10)),
            ..working()
        };
        assert_ne!(headline(&answered).title, "Google 服务器繁忙");

        let refused_after = Facts {
            overloaded: Some((secs(300), String::new())),
            refusal: Some((secs(30), 4)),
            ..working()
        };
        assert_ne!(headline(&refused_after).title, "Google 服务器繁忙");
    }

    /// A 401 that keeps coming is a credential Google no longer accepts: the
    /// card asks for a re-login; a single one is a token refreshed late.
    #[test]
    fn repeated_401_asks_to_sign_in_again() {
        let bad = Facts {
            unauthorized: Some((secs(30), 12)),
            ..working()
        };
        let h = headline(&bad);
        assert_eq!(h.title, "需要重新登录 Antigravity");
        assert!(h.detail.contains("退出账号"), "{}", h.detail);

        let once = Facts {
            unauthorized: Some((secs(30), 1)),
            ..working()
        };
        assert_ne!(headline(&once).title, h.title);

        let answered = Facts {
            unauthorized: Some((secs(30), 12)),
            answer: Some(secs(5)),
            ..working()
        };
        assert_ne!(headline(&answered).title, h.title);
    }
}
