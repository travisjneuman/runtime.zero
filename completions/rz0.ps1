# runtime.zero PowerShell completion
Register-ArgumentCompleter -Native -CommandName rz0 -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $tokens = @($commandAst.CommandElements | ForEach-Object { $_.Extent.Text })
    $commands = @('doctor','config','apps','cache','leftovers','recovery','restore','integrity','uninstall','modules','store','scan','monitor','toolchain','report','release','updates','completions','help','version')
    $managers = @('homebrew-formula','homebrew-cask','macports','winget','apt','dnf','pacman','zypper','snap','flatpak')
    $candidates = if ($tokens.Count -le 2) {
        $commands + @('--tui','--no-tui','--json','--color','--version','--help')
    } elseif ($tokens[-2] -eq '--format') {
        if ($tokens[1] -eq 'store') { @('json') } else { @('text','json') }
    } elseif ($tokens[-2] -eq '--color') {
        @('auto','always','never')
    } elseif ($tokens[-2] -eq '--manager') {
        $managers
    } elseif ($tokens[1] -eq 'completions') {
        @('bash','zsh','fish','powershell')
    } elseif ($tokens[1] -eq 'updates') {
        @('--dry-run','--fixture','--manager-output','--manager','--executable','--probe','--allow-network-read','--plan','--queue','--apply','--action','--all','--confirm','--challenge-issued-unix-seconds','--accept-no-rollback','--allow-network-write','--recovery-status','--transaction','--format','--json','--help')
    } elseif ($tokens[1] -eq 'release') {
        @('status','--assessment','--format','--json','--help')
    } elseif ($tokens[1] -eq 'uninstall') {
        @('plan','--executable','--format','--json','--help')
    } elseif ($tokens[1] -eq 'store') {
        @('plan','status','init','--store-root','--dry-run','--yes','--format','--json','--help')
    } elseif ($tokens[1] -eq 'modules') {
        @('status','validate','install','invoke','lifecycle-plan','trust','--from','--store-root','--module-id','--from-state','--to-state','--from-version','--to-version','--transition-id','--developer-trial','--signature','--trusted-test-key','--challenge-issued-unix-seconds','--confirm','--dry-run','--apply','--format','--json','--help')
    } elseif ($tokens[1] -eq 'cache') {
        @('--dry-run','--fixture','--plan','--apply','--path','--challenge-issued-unix-seconds','--confirm','--format','--json','--help')
    } elseif ($tokens[1] -eq 'leftovers') {
        @('--dry-run','--fixture','--plan','--apply','--path','--challenge-issued-unix-seconds','--confirm','--format','--json','--help')
    } elseif ($tokens[1] -eq 'restore') {
        @('--dry-run','--apply','--plan-id','--challenge-issued-unix-seconds','--confirm','--format','--json','--help')
    } elseif ($tokens[1] -eq 'recovery') {
        @('--dry-run','--format','--json','--help')
    } elseif ($tokens[1] -eq 'integrity') {
        @('--dry-run','--fixture','--format','--json','--help')
    } elseif ($tokens[1] -eq 'scan') {
        @('--dry-run','--include-raw-paths','--format','--json','--help')
    } else {
        @('--format','--json','--help')
    }

    $candidates |
        Where-Object { $_ -like "$wordToComplete*" } |
        Sort-Object -Unique |
        ForEach-Object {
            [System.Management.Automation.CompletionResult]::new($_, $_, 'ParameterValue', $_)
        }
}
