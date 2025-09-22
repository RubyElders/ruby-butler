# Integration Tests for Ruby Butler Runtime Commands
# Tests runtime command functionality with actual Ruby installation

BeforeAll {
    $Script:RbPath = $env:RB_TEST_PATH
    if (-not $Script:RbPath) {
        throw "RB_TEST_PATH environment variable not set. Run Setup.ps1 first."
    }
}

Describe "Ruby Butler - Runtime Command Integration" {
    Context "Runtime Information Display" {
        It "Shows runtime information successfully" {
            $Output = & $Script:RbPath runtime 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "Ruby Environment Survey|Environment Ready|CRuby"
        }
        
        It "Shows runtime information with rt alias successfully" {
            $Output = & $Script:RbPath rt 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "Ruby Environment Survey|Environment Ready|CRuby"
        }
        
        It "Runtime shows Ruby version information" {
            $Output = & $Script:RbPath runtime 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "\d+\.\d+\.\d+"
        }
    }
}
