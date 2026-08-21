# runtime.zero bash completion
_rz0_complete() {
    local current previous command
    COMPREPLY=()
    current="${COMP_WORDS[COMP_CWORD]}"
    previous="${COMP_WORDS[COMP_CWORD-1]}"
    command="${COMP_WORDS[1]}"

    case "$previous" in
        --format)
            if [[ $command == store ]]; then
                COMPREPLY=( $(compgen -W 'json' -- "$current") )
            else
                COMPREPLY=( $(compgen -W 'text json' -- "$current") )
            fi
            return
            ;;
        --color) COMPREPLY=( $(compgen -W 'auto always never' -- "$current") ); return ;;
        --manager) COMPREPLY=( $(compgen -W 'homebrew-formula homebrew-cask macports winget apt dnf pacman zypper snap flatpak' -- "$current") ); return ;;
        --fixture|--manager-output|--executable|--from|--store-root) COMPREPLY=( $(compgen -f -- "$current") ); return ;;
        completions) COMPREPLY=( $(compgen -W 'bash zsh fish powershell' -- "$current") ); return ;;
    esac

    if [[ $COMP_CWORD -eq 1 ]]; then
        COMPREPLY=( $(compgen -W 'doctor apps cache leftovers integrity uninstall modules store scan monitor toolchain report release updates completions help version --tui --no-tui --json --color --version --help' -- "$current") )
        return
    fi
    case "$command" in
        doctor|apps|monitor|toolchain|report) COMPREPLY=( $(compgen -W '--format --json --help' -- "$current") ) ;;
        release) COMPREPLY=( $(compgen -W 'status --assessment --format --json --help' -- "$current") ) ;;
        cache) COMPREPLY=( $(compgen -W '--dry-run --fixture --format --json --help' -- "$current") ) ;;
        leftovers) COMPREPLY=( $(compgen -W '--dry-run --fixture --format --json --help' -- "$current") ) ;;
        integrity) COMPREPLY=( $(compgen -W '--dry-run --fixture --format --json --help' -- "$current") ) ;;
        uninstall) COMPREPLY=( $(compgen -W 'plan --executable --format --json --help' -- "$current") ) ;;
        modules) COMPREPLY=( $(compgen -W 'validate install lifecycle-plan --from --module-id --from-state --to-state --from-version --to-version --transition-id --dry-run --format --json --help' -- "$current") ) ;;
        store) COMPREPLY=( $(compgen -W 'plan status init --store-root --dry-run --yes --format --json --help' -- "$current") ) ;;
        scan) COMPREPLY=( $(compgen -W '--dry-run --include-raw-paths --format --json --help' -- "$current") ) ;;
        updates) COMPREPLY=( $(compgen -W '--dry-run --fixture --manager-output --manager --executable --probe --allow-network-read --plan --queue --apply --action --all --confirm --challenge-issued-unix-seconds --accept-no-rollback --allow-network-write --recovery-status --transaction --format --json --help' -- "$current") ) ;;
        *) COMPREPLY=( $(compgen -W '--help --version --color' -- "$current") ) ;;
    esac
}
complete -F _rz0_complete rz0
