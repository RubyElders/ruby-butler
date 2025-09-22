# Integration Tests for Ruby Butler Ruby Command Execution
# Tests Ruby command execution through Ruby Butler

BeforeAll {
    $Script:RbPath = $env:RB_TEST_PATH
    if (-not $Script:RbPath) {
        throw "RB_TEST_PATH environment variable not set. Run Setup.ps1 first."
    }
}

Describe "Ruby Butler - Ruby Command Execution Integration" {
    Context "Ruby Version Commands" {
        It "Executes ruby -v command successfully" {
            $Output = & $Script:RbPath x ruby -v 2>&1
            $LASTEXITCODE | Should -Be 0
            $Output | Should -Match "ruby \d+\.\d+\.\d+"
        }
        
        It "Executes ruby --version command successfully" {
            $Output = & $Script:RbPath x ruby --version 2>&1
            $LASTEXITCODE | Should -Be 0
            $Output | Should -Match "ruby \d+\.\d+\.\d+"
        }
    }
    
    Context "Ruby Script Execution" {
        It "Executes simple Ruby script" {
            $Output = & $Script:RbPath x ruby -e "puts 'Hello from Ruby Butler'" 2>&1
            $LASTEXITCODE | Should -Be 0
            $Output | Should -Match "Hello from Ruby Butler"
        }
        
        It "Executes Ruby script with variables" {
            $Output = & $Script:RbPath x ruby -e 'name = "Butler"; puts "Hello from #{name}"' 2>&1
            $LASTEXITCODE | Should -Be 0
            $Output | Should -Match "Hello from Butler"
        }
        
        It "Executes Ruby script with basic math" {
            $Output = & $Script:RbPath x ruby -e "puts 2 + 2" 2>&1
            $LASTEXITCODE | Should -Be 0
            $Output | Should -Match "4"
        }
    }
}
