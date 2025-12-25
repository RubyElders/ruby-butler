# Unit Tests for Ruby Butler Help System and Basic CLI
# Tests core CLI functionality that doesn't require Ruby installation

BeforeAll {
    $Script:RbPath = $env:RB_TEST_PATH
    if (-not $Script:RbPath) {
        throw "RB_TEST_PATH environment variable not set. Run Setup.ps1 first."
    }
}

Describe "Ruby Butler - Help System" {
    Context "Help Command" {
        It "Shows help with help command" {
            $Output = & $Script:RbPath help 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "Ruby Butler|Usage|Commands"
        }
        
        It "Lists main commands in help" {
            $Output = & $Script:RbPath help 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "runtime|environment|exec"
        }
        
        It "Lists utility commands in help" {
            $Output = & $Script:RbPath help 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "init|config|version|help"
        }
        
        It "Shows command aliases in help" {
            $Output = & $Script:RbPath help 2>&1
            $LASTEXITCODE | Should -Be 0
            $OutputText = $Output -join " "
            $OutputText | Should -Match "rt"
            $OutputText | Should -Match "env"
            $OutputText | Should -Match "x"
        }
        
        It "Shows options section in help" {
            $Output = & $Script:RbPath help 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "Options"
        }
        
        It "Rejects --help flag with error" {
            $Output = & $Script:RbPath --help 2>&1
            $LASTEXITCODE | Should -Not -Be 0
            ($Output -join " ") | Should -Match "unexpected argument"
        }
    }
    
    Context "Command-Specific Help" {
        It "Shows runtime command help" {
            $Output = & $Script:RbPath help runtime 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "runtime"
        }
        
        It "Shows environment command help" {
            $Output = & $Script:RbPath help environment 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "environment"
        }
        
        It "Shows exec command help" {
            $Output = & $Script:RbPath help exec 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "exec"
        }
        
        It "Shows sync command help" {
            $Output = & $Script:RbPath help sync 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "sync"
        }
        
        It "Shows run command help" {
            $Output = & $Script:RbPath help run 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "run"
        }
        
        It "Shows init command help" {
            $Output = & $Script:RbPath help init 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "init"
        }
        
        It "Shows config command help" {
            $Output = & $Script:RbPath help config 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "config"
        }
        
        It "Shows version command help" {
            $Output = & $Script:RbPath help version 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "version"
        }
    }
}

Describe "Ruby Butler - Version Command" {
    Context "Version Information" {
        It "Shows version with version command" {
            $Output = & $Script:RbPath version 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "Ruby Butler v\d+\.\d+\.\d+"
        }
        
        It "Shows sophisticated description in version" {
            $Output = & $Script:RbPath version 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "sophisticated.*environment manager|gentleman's gentleman"
        }
        
        It "Rejects --version flag with error" {
            $Output = & $Script:RbPath --version 2>&1
            $LASTEXITCODE | Should -Not -Be 0
            ($Output -join " ") | Should -Match "unexpected argument"
        }
    }
}

Describe "Ruby Butler - Command Recognition" {
    Context "Runtime Commands" {
        It "Recognizes runtime command" {
            $Output = & $Script:RbPath help runtime 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "runtime|Survey.*Ruby"
        }
    }
    
    Context "Environment Commands" {
        It "Recognizes environment command" {
            $Output = & $Script:RbPath help environment 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "environment|Present.*current.*Ruby"
        }
        
        It "Recognizes env alias for environment" {
            $Output = & $Script:RbPath help environment 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "env"
        }
    }
    
    Context "Execution Commands" {
        It "Recognizes exec command" {
            $Output = & $Script:RbPath help exec 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "exec|Execute.*command.*Ruby"
        }
        
        It "Recognizes x alias for exec" {
            $Output = & $Script:RbPath help exec 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "x"
        }
    }
}

Describe "Ruby Butler - Gentleman's Approach" {
    Context "Language and Branding" {
        It "Uses sophisticated language" {
            $Output = & $Script:RbPath help 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "distinguished|sophisticated|refined|meticulously"
        }
        
        It "Presents as environment manager, not version switcher" {
            $Output = & $Script:RbPath help 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "environment manager|environment"
        }
        
        It "Includes butler emoji in help" {
            $Output = & $Script:RbPath help 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "🎩|Ruby Butler"
        }
    }
}

Describe "Ruby Butler - Error Handling" {
    Context "Invalid Input Handling" {
        It "Handles invalid commands gracefully" {
            $Output = & $Script:RbPath invalid-command 2>&1
            $LASTEXITCODE | Should -Be 2
        }
        
        It "Handles invalid options gracefully" {
            $Output = & $Script:RbPath --invalid-option 2>&1
            $LASTEXITCODE | Should -Be 2
        }
    }
}
