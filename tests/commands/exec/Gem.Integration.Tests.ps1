# Integration Tests for Ruby Butler Gem Command Execution
# Tests gem command execution through Ruby Butler

BeforeAll {
    $Script:RbPath = $env:RB_TEST_PATH
    if (-not $Script:RbPath) {
        throw "RB_TEST_PATH environment variable not set. Run Setup.ps1 first."
    }
}

Describe "Ruby Butler - Gem Command Execution Integration" {
    Context "Gem Information Commands" {
        It "Executes gem env command successfully" {
            $Output = & $Script:RbPath x gem env 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "RubyGems Environment|RUBYGEMS VERSION|INSTALLATION DIRECTORY"
        }
        
        It "Executes gem --version command successfully" {
            $Output = & $Script:RbPath x gem --version 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "\d+\.\d+\.\d+"
        }
        
        It "Executes gem help command successfully" {
            $Output = & $Script:RbPath x gem help 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "RubyGems|build|install|list"
        }
    }
    
    Context "Gem List Commands" {
        It "Executes gem list command successfully" {
            $Output = & $Script:RbPath x gem list 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "LOCAL GEMS|abbrev|bundler|default:"
        }
        
        It "Executes gem list --local command successfully" {
            $Output = & $Script:RbPath x gem list --local 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "LOCAL GEMS|abbrev|bundler|default:"
        }
    }
    
    Context "Gem Query Commands" {
        It "Executes gem which bundler command successfully" {
            & $Script:RbPath x gem which bundler 2>&1 | Out-Null
            # gem which might not work for bundler, but should not fail with error
            $LASTEXITCODE | Should -BeIn @(0, 1)
        }
        
        It "Executes gem specification bundler command successfully" {
            $Output = & $Script:RbPath x gem specification bundler 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "name.*bundler|version.*2\.\d+\.\d+"
        }
    }
}
