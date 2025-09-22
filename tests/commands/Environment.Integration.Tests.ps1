# Integration Tests for Ruby Butler Environment Commands
# Tests environment command functionality with actual Ruby installation

BeforeAll {
    $Script:RbPath = $env:RB_TEST_PATH
    if (-not $Script:RbPath) {
        throw "RB_TEST_PATH environment variable not set. Run Setup.ps1 first."
    }
}

Describe "Ruby Butler - Environment Command Integration" {
    Context "Environment Information Display" {
        It "Shows environment details successfully" {
            $Output = & $Script:RbPath environment 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "Your Current Ruby Environment|Environment Summary|Active Ruby"
        }
        
        It "Shows environment details with env alias successfully" {
            $Output = & $Script:RbPath env 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "Your Current Ruby Environment|Environment Summary|Active Ruby"
        }
        
        It "Environment shows path information" {
            $Output = & $Script:RbPath environment 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "Executable paths|\.rubies|bin"
        }
        
        It "Environment shows gem configuration" {
            $Output = & $Script:RbPath environment 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "Gem home|Gem libraries"
        }
    }
}
