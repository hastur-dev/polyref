$ErrorActionPreference = "Stop"
cargo build --release --workspace

$hooksDir = if ($env:CLAUDE_HOOKS_DIR) { $env:CLAUDE_HOOKS_DIR } else { ".claude" }
New-Item -ItemType Directory -Force -Path $hooksDir | Out-Null

$hookBin = ".\target\release\crate-ref-hook.exe"
$config = @{
    hooks = @{
        PostToolUse = @(
            @{
                matcher = "Write|Edit|MultiEdit"
                hooks = @(@{ type = "command"; command = $hookBin })
            }
        )
    }
} | ConvertTo-Json -Depth 10

Set-Content "$hooksDir\hooks.json" $config
Write-Host "Hooks installed to $hooksDir\hooks.json"
