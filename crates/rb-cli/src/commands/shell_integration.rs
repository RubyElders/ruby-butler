use crate::Shell;
use colored::Colorize;
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
    println!("{}\n", "🎩 Available Shell Integrations".bold());
    println!("{}", "Shells:".bold());

    for integration in available_integrations() {
        println!(
            "  {:<12} {}",
            integration.shell_name.green(),
            integration.description
        );
    }

    println!("\n{}", "Installation:".bold());
    for integration in available_integrations() {
        println!(
            "  {:<12} {}",
            integration.shell_name.green(),
            integration.install_instruction
        );
    }
}

pub fn shell_integration_command(shell: Shell) -> Result<(), Box<dyn std::error::Error>> {
    match shell {
        Shell::Bash => {
            generate_bash_shim();
            if std::io::stdout().is_terminal() {
                print_bash_instructions();
            }
        }
    }

    Ok(())
}

fn generate_bash_shim() {
    print!(
        r#"# Ruby Butler dynamic completion shim
_rb_completion() {{
    local cur prev words cword
    _init_completion || return

    # Call rb to get context-aware completions
    local completions
    completions=$(rb __bash_complete "${{COMP_LINE}}" "${{COMP_POINT}}" 2>/dev/null)
    
    if [ -n "$completions" ]; then
        COMPREPLY=($(compgen -W "$completions" -- "$cur"))
        # Bash will automatically add space for single completion
    else
        # No rb completions, fall back to default bash completion (files/dirs)
        compopt -o default
        COMPREPLY=()
    fi
}}

complete -F _rb_completion rb
"#
    );
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
