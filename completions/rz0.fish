# runtime.zero fish completion
complete -c rz0 -f
complete -c rz0 -n '__fish_use_subcommand' -a doctor -d 'Show privacy-safe diagnostics'
complete -c rz0 -n '__fish_use_subcommand' -a apps -d 'List path-free installed software'
complete -c rz0 -n '__fish_use_subcommand' -a cache -d 'Review bounded cache evidence without mutation'
complete -c rz0 -n '__fish_use_subcommand' -a leftovers -d 'Review bounded runtime.zero-owned evidence without mutation'
complete -c rz0 -n '__fish_use_subcommand' -a integrity -d 'Review explicit digest evidence without remediation'
complete -c rz0 -n '__fish_use_subcommand' -a uninstall -d 'Build a read-only uninstall plan'
complete -c rz0 -n '__fish_use_subcommand' -a modules -d 'Inspect module manifests and plans'
complete -c rz0 -n '__fish_use_subcommand' -a store -d 'Inspect or initialize the local store'
complete -c rz0 -n '__fish_use_subcommand' -a scan -d 'Collect bounded dry-run inventory'
complete -c rz0 -n '__fish_use_subcommand' -a monitor -d 'Show a native system snapshot'
complete -c rz0 -n '__fish_use_subcommand' -a toolchain -d 'Show Rust, AI, and developer toolchain posture'
complete -c rz0 -n '__fish_use_subcommand' -a report -d 'Build a privacy-reviewed local summary'
complete -c rz0 -n '__fish_use_subcommand' -a release -d 'Inspect a release-acceptance assessment'
complete -c rz0 -n '__fish_use_subcommand' -a updates -d 'Review, apply, or assess manager updates'
complete -c rz0 -n '__fish_use_subcommand' -a completions -d 'Print shell completion source'
complete -c rz0 -n '__fish_use_subcommand' -a help -d 'Show help'
complete -c rz0 -n '__fish_use_subcommand' -a version -d 'Show version'
complete -c rz0 -l color -xa 'auto always never'
complete -c rz0 -l help -s h
complete -c rz0 -l version -s V

for command in doctor apps monitor toolchain report
    complete -c rz0 -n "__fish_seen_subcommand_from $command" -l format -xa 'text json'
    complete -c rz0 -n "__fish_seen_subcommand_from $command" -l json
end
complete -c rz0 -n '__fish_seen_subcommand_from release' -a status -d 'Inspect one bounded assessment'
complete -c rz0 -n '__fish_seen_subcommand_from release' -l assessment -rF
complete -c rz0 -n '__fish_seen_subcommand_from release' -l format -xa 'text json'
complete -c rz0 -n '__fish_seen_subcommand_from release' -l json
complete -c rz0 -n '__fish_seen_subcommand_from scan' -l dry-run
complete -c rz0 -n '__fish_seen_subcommand_from scan' -l include-raw-paths
complete -c rz0 -n '__fish_seen_subcommand_from scan' -l format -xa 'text json'
complete -c rz0 -n '__fish_seen_subcommand_from scan' -l json
complete -c rz0 -n '__fish_seen_subcommand_from uninstall' -a plan
complete -c rz0 -n '__fish_seen_subcommand_from uninstall' -l executable -rF
complete -c rz0 -n '__fish_seen_subcommand_from uninstall' -l format -xa 'text json'
complete -c rz0 -n '__fish_seen_subcommand_from uninstall' -l json
complete -c rz0 -n '__fish_seen_subcommand_from modules' -a 'validate install lifecycle-plan'
complete -c rz0 -n '__fish_seen_subcommand_from modules' -l from -rF
complete -c rz0 -n '__fish_seen_subcommand_from modules' -l module-id
complete -c rz0 -n '__fish_seen_subcommand_from modules' -l from-state -xa 'absent staged installed_inactive active degraded quarantined'
complete -c rz0 -n '__fish_seen_subcommand_from modules' -l to-state -xa 'absent staged installed_inactive active degraded quarantined'
complete -c rz0 -n '__fish_seen_subcommand_from modules' -l from-version
complete -c rz0 -n '__fish_seen_subcommand_from modules' -l to-version
complete -c rz0 -n '__fish_seen_subcommand_from modules' -l transition-id
complete -c rz0 -n '__fish_seen_subcommand_from modules' -l dry-run
complete -c rz0 -n '__fish_seen_subcommand_from modules' -l format -xa 'text json'
complete -c rz0 -n '__fish_seen_subcommand_from modules' -l json
complete -c rz0 -n '__fish_seen_subcommand_from cache' -l dry-run
complete -c rz0 -n '__fish_seen_subcommand_from cache' -l fixture -rF
complete -c rz0 -n '__fish_seen_subcommand_from cache' -l plan
complete -c rz0 -n '__fish_seen_subcommand_from cache' -l apply
complete -c rz0 -n '__fish_seen_subcommand_from cache' -l path -rF
complete -c rz0 -n '__fish_seen_subcommand_from cache' -l challenge-issued-unix-seconds -r
complete -c rz0 -n '__fish_seen_subcommand_from cache' -l confirm -r
complete -c rz0 -n '__fish_seen_subcommand_from cache' -l format -xa 'text json'
complete -c rz0 -n '__fish_seen_subcommand_from cache' -l json
complete -c rz0 -n '__fish_seen_subcommand_from leftovers' -l dry-run
complete -c rz0 -n '__fish_seen_subcommand_from leftovers' -l fixture -rF
complete -c rz0 -n '__fish_seen_subcommand_from leftovers' -l plan
complete -c rz0 -n '__fish_seen_subcommand_from leftovers' -l apply
complete -c rz0 -n '__fish_seen_subcommand_from leftovers' -l path -rF
complete -c rz0 -n '__fish_seen_subcommand_from leftovers' -l challenge-issued-unix-seconds -r
complete -c rz0 -n '__fish_seen_subcommand_from leftovers' -l confirm -r
complete -c rz0 -n '__fish_seen_subcommand_from leftovers' -l format -xa 'text json'
complete -c rz0 -n '__fish_seen_subcommand_from leftovers' -l json
complete -c rz0 -n '__fish_seen_subcommand_from integrity' -l dry-run
complete -c rz0 -n '__fish_seen_subcommand_from integrity' -l fixture -rF
complete -c rz0 -n '__fish_seen_subcommand_from integrity' -l format -xa 'text json'
complete -c rz0 -n '__fish_seen_subcommand_from integrity' -l json
complete -c rz0 -n '__fish_seen_subcommand_from store' -a 'plan status init'
complete -c rz0 -n '__fish_seen_subcommand_from store' -l store-root -rF
complete -c rz0 -n '__fish_seen_subcommand_from store' -l dry-run
complete -c rz0 -n '__fish_seen_subcommand_from store' -l yes
complete -c rz0 -n '__fish_seen_subcommand_from store' -l format -xa 'json'
complete -c rz0 -n '__fish_seen_subcommand_from store' -l json
complete -c rz0 -n '__fish_seen_subcommand_from completions' -a 'bash zsh fish powershell'

set -l managers homebrew-formula homebrew-cask macports winget apt dnf pacman zypper snap flatpak
complete -c rz0 -n '__fish_seen_subcommand_from updates' -l manager -xa "$managers"
complete -c rz0 -n '__fish_seen_subcommand_from updates' -l executable -rF
complete -c rz0 -n '__fish_seen_subcommand_from updates' -l fixture -rF
complete -c rz0 -n '__fish_seen_subcommand_from updates' -l manager-output -rF
for option in dry-run probe allow-network-read plan queue apply all allow-network-write accept-no-rollback recovery-status json
    complete -c rz0 -n '__fish_seen_subcommand_from updates' -l $option
end
complete -c rz0 -n '__fish_seen_subcommand_from updates' -l action -r
complete -c rz0 -n '__fish_seen_subcommand_from updates' -l confirm -r
complete -c rz0 -n '__fish_seen_subcommand_from updates' -l challenge-issued-unix-seconds -r
complete -c rz0 -n '__fish_seen_subcommand_from updates' -l transaction -r
complete -c rz0 -n '__fish_seen_subcommand_from updates' -l format -xa 'text json'
