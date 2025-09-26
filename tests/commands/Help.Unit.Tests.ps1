# Unit Tests for Ruby Butler Help System and Basic CLI
# Tests core CLI functionality that doesn't require Ruby installation

BeforeAll {
    $Script:RbPath = $env:RB_TEST_PATH
    if (-not $Script:RbPath) {
        throw "RB_TEST_PATH environment variable not set. Run Setup.ps1 first."
    }
}

Describe "Ruby Butler - Help System" {
    Context "Help Command Options" {
        It "Shows help with --help flag" {
            $Output = & $Script:RbPath --help 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "Ruby Butler|Usage|Commands"
        }
        
        It "Shows help with -h flag" {
            $Output = & $Script:RbPath -h 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "Ruby Butler|Usage|Commands"
        }
        
        It "Lists main commands in help" {
            $Output = & $Script:RbPath --help 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "runtime|environment|exec"
        }
    }
    
    Context "Version Information" {
        It "Shows version with --version flag" {
            $Output = & $Script:RbPath --version 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "Ruby Butler v\d+\.\d+\.\d+"
        }
        
        It "Shows version with -V flag" {
            $Output = & $Script:RbPath -V 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "Ruby Butler v\d+\.\d+\.\d+"
        }
    }
}

Describe "Ruby Butler - Command Recognition" {
    Context "Runtime Commands" {
        It "Recognizes runtime command" {
            $Output = & $Script:RbPath runtime --help 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "runtime|Survey.*Ruby"
        }
        
        It "Recognizes rt alias for runtime" {
            $Output = & $Script:RbPath rt --help 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "runtime|Survey.*Ruby"
        }
    }
    
    Context "Environment Commands" {
        It "Recognizes environment command" {
            $Output = & $Script:RbPath environment --help 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "environment|Present.*current.*Ruby"
        }
        
        It "Recognizes env alias for environment" {
            $Output = & $Script:RbPath env --help 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "environment|Present.*current.*Ruby"
        }
    }
    
    Context "Execution Commands" {
        It "Recognizes exec command" {
            $Output = & $Script:RbPath exec --help 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "exec|Execute.*command.*Ruby"
        }
        
        It "Recognizes x alias for exec" {
            $Output = & $Script:RbPath x --help 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "exec|Execute.*command.*Ruby"
        }
    }
}

Describe "Ruby Butler - Gentleman's Approach" {
    Context "Language and Branding" {
        It "Uses sophisticated language" {
            $Output = & $Script:RbPath --help 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "distinguished|sophisticated|refined|gentleman"
        }
        
        It "Presents as environment manager, not version switcher" {
            $Output = & $Script:RbPath --help 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "environment manager|environment|orchestrates"
        }
        
        It "Includes RubyElders branding" {
            $Output = & $Script:RbPath --help 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "RubyElders"
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
