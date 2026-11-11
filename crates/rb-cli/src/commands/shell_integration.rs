use crate::Shell;
use std::io::IsTerminal;

/// Metadata about a shell integration
pub struct ShellIntegration {
    pub name: &'static str,
    pub shell_name: &'static str,
    pub shell: Shell,
    pub description: &'static str,
    pub install_instruction: &'static str,
}

/// All available shell integrations
pub fn available_integrations() -> Vec<ShellIntegration> {
    vec![ShellIntegration {
        name: "Bash Completion",
        shell_name: "bash",
        shell: Shell::Bash,
        description: "Dynamic command completion for Bash shell",
        install_instruction: "Add to ~/.bashrc: eval \"$(rb shell-integration bash)\"",
    }]
}

/// Show all available shell integrations with installation instructions
pub fn show_available_integrations() {
    available_with_output(&crate::output::Output::default()).expect("writing shell integrations");
}

pub(crate) fn available_with_output(
    output: &crate::output::Output,
) -> Result<(), rb_core::butler::ButlerError> {
    let report = available_integrations()
        .into_iter()
        .map(|value| IntegrationReport {
            name: value.name,
            shell: value.shell_name,
            description: value.description,
            instruction: value.install_instruction,
        })
        .collect::<Vec<_>>();
    output.emit("shell-integration", &report, || {
        crate::presentation::shell::document(&report)
    })
}
#[derive(serde::Serialize)]
pub(crate) struct IntegrationReport {
    pub name: &'static str,
    pub shell: &'static str,
    pub description: &'static str,
    pub instruction: &'static str,
}

pub fn shell_integration_command(shell: Shell) -> Result<(), Box<dyn std::error::Error>> {
    shell_with_output(shell, &crate::output::Output::default()).map_err(Into::into)
}
pub(crate) fn shell_with_output(
    shell: Shell,
    output: &crate::output::Output,
) -> Result<(), rb_core::butler::ButlerError> {
    let shell_name = match shell {
        Shell::Bash => "bash",
    };
    let source = bash_source();
    output.emit(
        "shell-integration",
        &ShellSource {
            shell: shell_name,
            source: &source,
        },
        || crate::output::document::Document::verbatim(source.clone()),
    )?;
    if std::io::stdout().is_terminal() {
        print_bash_instructions();
    }
    Ok(())
}

#[derive(serde::Serialize)]
struct ShellSource<'a> {
    shell: &'static str,
    source: &'a str,
}

fn bash_source() -> String {
    r#"# Ruby Butler dynamic completion shim
_rb_completion() {
    local cur prev words cword
    _init_completion || return

    # Call rb to get context-aware completions
    local completions
    completions=$(rb __bash_complete "${COMP_LINE}" "${COMP_POINT}" 2>/dev/null)

    if [ -n "$completions" ]; then
        # Check if all completions are directories (end with /)
        # If so, don't add space to allow continued navigation
        local all_dirs=true
        while IFS= read -r line; do
            if [[ ! "$line" =~ /$ ]]; then
                all_dirs=false
                break
            fi
        done <<< "$completions"

        if [ "$all_dirs" = true ]; then
            compopt -o nospace
        fi

        COMPREPLY=($(compgen -W "$completions" -- "$cur"))
    else
        # No rb completions - check if we're completing a directory-only flag
        # For -C, -R, -G: don't fall back to default (which would show files)
        case "$prev" in
            -C|--work-dir|-R|--rubies-dir|-G|--gem-home)
                # Return empty completions (no files for directory flags)
                COMPREPLY=()
                ;;
            *)
                # Fall back to default bash completion for other cases
                compopt -o default
                COMPREPLY=()
                ;;
        esac
    fi
}

complete -F _rb_completion rb
"#
    .to_string()
}

fn print_bash_instructions() {
    eprintln!("\n# 🎩 Ruby Butler Shell Integration");
    eprintln!("#");
    eprintln!("# To enable completions, add to your ~/.bashrc:");
    eprintln!("#   eval \"$(rb shell-integration bash)\"");
    eprintln!("#");
    eprintln!("# This generates completions on-the-fly, ensuring they stay current");
    eprintln!("# with your installed version. The generation is instantaneous.");
}
