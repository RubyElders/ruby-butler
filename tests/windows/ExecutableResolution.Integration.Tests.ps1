# Integration Tests for Ruby Butler Windows Executable Resolution
# Tests Windows-specific executable resolution functionality

BeforeAll {
    $Script:RbPath = $env:RB_TEST_PATH
    if (-not $Script:RbPath) {
        throw "RB_TEST_PATH environment variable not set. Run Setup.ps1 first."
    }
}

Describe "Ruby Butler - Windows Executable Resolution Integration" {
    Context "Gem Command Resolution" {
        It "Resolves gem command without .cmd extension" {
            # Test that we can call 'gem' without explicitly using 'gem.cmd'
            $Output = & $Script:RbPath x gem env 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "RubyGems Environment|RUBYGEMS VERSION"
        }
        
        It "Resolves gem --version without .cmd extension" {
            $Output = & $Script:RbPath x gem --version 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "\d+\.\d+\.\d+"
        }
    }
    
    Context "IRB Command Resolution" {
        It "Resolves irb command without .cmd extension" {
            # Test IRB help (quick exit)
            $Output = & $Script:RbPath x irb --help 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "Usage.*irb|Don't initialize from configuration"
        }
    }
    
    Context "Ruby Command Resolution" {
        It "Resolves ruby command without .exe extension" {
            $Output = & $Script:RbPath x ruby --version 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "ruby \d+\.\d+\.\d+"
        }
        
        It "Resolves ruby -v without .exe extension" {
            $Output = & $Script:RbPath x ruby -v 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "ruby \d+\.\d+\.\d+"
        }
    }
    
    Context "Bundle Command Resolution" {
        It "Resolves bundle command without .bat/.cmd extension" {
            $Output = & $Script:RbPath x bundle --version 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "Bundler version \d+\.\d+\.\d+"
        }
    }
    
    Context "Ruby Ecosystem Command Validation" {
        It "Handles non-existent Ruby commands gracefully" {
            # Test that non-existent Ruby ecosystem commands fail with command not found error
            $Output = & $Script:RbPath x nonexistent-ruby-command-12345 2>&1
            $LASTEXITCODE | Should -Be 127
            ($Output -join " ") | Should -Match "sincerest apologies.*command.*appears to be.*entirely absent"
        }
    }
}
