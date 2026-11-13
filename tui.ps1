# Antigravity Unlocker - terminal mode in one command (Windows 10/11):
#
#   PowerShell:  irm https://raw.githubusercontent.com/sjgld/Antigravity/zh-CN/tui.ps1 | iex
#   cmd:         powershell -NoProfile -c "irm https://raw.githubusercontent.com/sjgld/Antigravity/zh-CN/tui.ps1 | iex"
#
# Fetches the latest release from GitHub into %LOCALAPPDATA%\AGUnlocker\AG.exe and
# runs it right in this console. Running the same command again downloads only
# when there is a newer version. Start it from an elevated console for the DNS
# half of the bypass; otherwise the program offers a restart with admin rights.
#
# ASCII only, and no BOM (G60): `irm | iex` keeps a BOM as U+FEFF, which the
# PowerShell 5.1 parser rejects, while without a BOM a *file* is read as ANSI by
# 5.1 and any non-ASCII byte can turn into a quote character. ASCII is the one
# encoding both ways read the same.

# In a script block, so `iex` leaves nothing behind in the caller's session.
& {
    # ASCII source keeps both PowerShell 5.1 file loading and irm | iex safe.
    function Zh([string]$Text) {
        [regex]::Replace($Text, '\u([0-9a-fA-F]{4})', {
            param($m) [string][char][Convert]::ToInt32($m.Groups[1].Value, 16)
        })
    }
    $ErrorActionPreference = 'Stop'
    # Invoke-WebRequest's progress bar makes a 15 MB download take minutes on
    # Windows PowerShell 5.1.
    $ProgressPreference = 'SilentlyContinue'
    # An older Windows 10 can still default to TLS 1.0, which GitHub refuses.
    [Net.ServicePointManager]::SecurityProtocol =
        [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

    $repo = 'sjgld/Antigravity'
    $dir = Join-Path $env:LOCALAPPDATA 'AGUnlocker'
    $exe = Join-Path $dir 'AG.exe'
    $stamp = "$exe.version"

    # The newest tag from where /releases/latest redirects to: no API call, so
    # no rate limit.
    $tag = $null
    try {
        $r = Invoke-WebRequest -UseBasicParsing -Method Head "https://github.com/$repo/releases/latest"
        $uri = $r.BaseResponse.ResponseUri                                  # Windows PowerShell 5.1
        if (-not $uri) { $uri = $r.BaseResponse.RequestMessage.RequestUri } # PowerShell 7
        $tag = $uri.AbsoluteUri.TrimEnd('/').Split('/')[-1]
    } catch {}
    $ver = if ($tag) { $tag.TrimStart('v') } else { '' }

    if ($ver -notmatch '^[0-9._]+$') {
        if (-not (Test-Path $exe)) { throw (Zh '\u65e0\u6cd5\u4ece github.com/{0} \u83b7\u53d6\u6700\u65b0\u7248\u672c\u3002') -f $repo }
        Write-Host (Zh 'GitHub \u6682\u65e0\u54cd\u5e94\uff0c\u6b63\u5728\u542f\u52a8\u4e4b\u524d\u4e0b\u8f7d\u7684\u7248\u672c\u3002')
    } elseif (-not (Test-Path $exe) -or (Get-Content $stamp -ErrorAction SilentlyContinue) -ne $ver) {
        Write-Host ((Zh '\u6b63\u5728\u4e0b\u8f7d Antigravity \u89e3\u9501\u5668 {0}\u2026') -f $ver)
        New-Item -ItemType Directory -Force $dir | Out-Null
        # The tag spells a fourth number with a dot (v2.24.0.1), the asset with an
        # underscore (AG_2.24.0_1.exe): try both.
        $alt = $ver -replace '\.(\d+)$', '_$1'
        $got = $false
        foreach ($cand in @($ver, $alt)) {
            try {
                Invoke-WebRequest -UseBasicParsing "https://github.com/$repo/releases/download/$tag/AG_$cand.exe" -OutFile "$exe.part"
                $got = $true
                break
            } catch {}
        }
        if (-not $got) { throw ((Zh '\u7248\u672c {1} \u4e2d\u672a\u627e\u5230 AG_{0}.exe\u3002') -f $ver, $tag) }
        try {
            Move-Item -Force "$exe.part" $exe
            Set-Content -Path $stamp -Value $ver
        } catch {
            # The old copy is running (another window of it): it cannot be
            # replaced now, but it can still be started.
            Remove-Item -Force "$exe.part" -ErrorAction SilentlyContinue
            if (-not (Test-Path $exe)) { throw }
            Write-Host (Zh '\u65e7\u7248\u672c\u4ecd\u5728\u8fd0\u884c\uff0c\u6682\u65f6\u65e0\u6cd5\u66ff\u6362\uff0c\u5c06\u542f\u52a8\u73b0\u6709\u7248\u672c\u3002')
        }
    }

    # -Wait: the exe is a window program, which a console does not wait for by
    # itself. Waiting is what lets it draw here (--inline) without the prompt
    # taking every other keystroke.
    Start-Process -FilePath $exe -ArgumentList '--tui', '--inline' -Wait -NoNewWindow
}
