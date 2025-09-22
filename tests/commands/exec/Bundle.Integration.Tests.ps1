# Integration Tests for Ruby Butler Bundle Command Execution
# Tests bundle command execution through Ruby Butler

BeforeAll {
    $Script:RbPath = $env:RB_TEST_PATH
    if (-not $Script:RbPath) {
        throw "RB_TEST_PATH environment variable not set. Run Setup.ps1 first."
    }
}

Describe "Ruby Butler - Bundle Command Execution Integration" {
    Context "Bundle Information Commands" {
        It "Executes bundle --version command successfully" {
            $Output = & $Script:RbPath x bundle --version 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "Bundler version \d+\.\d+\.\d+"
        }
        
        It "Executes bundle help command successfully" {
            $Output = & $Script:RbPath x bundle help 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "BUNDLE COMMANDS|PRIMARY COMMANDS|install.*update.*exec"
        }
    }
}
