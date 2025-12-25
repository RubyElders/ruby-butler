# Integration Tests for Ruby Butler Version Command
# Tests version command functionality and output format

BeforeAll {
    $Script:RbPath = $env:RB_TEST_PATH
    if (-not $Script:RbPath) {
        throw "RB_TEST_PATH environment variable not set. Run Setup.ps1 first."
    }
}

Describe "Ruby Butler - Version Command" {
    Context "Version Display" {
        It "Shows version with version command" {
            $Output = & $Script:RbPath version 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "Ruby Butler v\d+\.\d+\.\d+"
        }
        
        It "Shows version number in proper format" {
            $Output = & $Script:RbPath version 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "v0\.\d+\.\d+"
        }
        
        It "Shows git commit hash" {
            $Output = & $Script:RbPath version 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "\([0-9a-f]+\)"
        }
        
        It "Shows sophisticated description" {
            $Output = & $Script:RbPath version 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "sophisticated.*environment manager|gentleman's gentleman"
        }
    }
    
    Context "Version Command Flags" {
        It "Rejects --version flag" {
            $Output = & $Script:RbPath --version 2>&1
            $LASTEXITCODE | Should -Not -Be 0
            ($Output -join " ") | Should -Match "unexpected argument"
        }
        
        It "Accepts -V flag as very verbose" {
            # -V is now the very verbose flag, not version
            $Output = & $Script:RbPath -V version 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "Ruby Butler|DEBUG"
        }
    }
    
    Context "Version with Verbose Flags" {
        It "Works with verbose flag" {
            $Output = & $Script:RbPath -v version 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "Ruby Butler"
        }
        
        It "Works with very verbose flag" {
            $Output = & $Script:RbPath -V version 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "Ruby Butler|DEBUG"
        }
    }
}
