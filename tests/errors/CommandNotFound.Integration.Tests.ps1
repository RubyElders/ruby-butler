# Integration Tests for Ruby Butler Command Not Found Error Handling
# Tests error handling when commands don't exist in the Ruby environment

BeforeAll {
    $Script:RbPath = $env:RB_TEST_PATH
    if (-not $Script:RbPath) {
        throw "RB_TEST_PATH environment variable not set. Run Setup.ps1 first."
    }
}

Describe "Ruby Butler - Command Not Found Error Handling" {
    Context "Nonexistent Command Error Messages" {
        It "Shows gentleman's butler error message for clearly fake command" {
            $FakeCommand = "definitely_does_not_exist_command_12345"
            
            $Output = & $Script:RbPath exec $FakeCommand 2>&1
            $LASTEXITCODE | Should -Be 127
            
            ($Output -join " ") | Should -Match "sincerest apologies.*command.*appears to be"
            ($Output -join " ") | Should -Match "entirely absent from.*distinguished Ruby environment"
            ($Output -join " ") | Should -Match "humble Butler.*meticulously searched"
            ($Output -join " ") | Should -Match "available paths.*gem installations"
            ($Output -join " ") | Should -Match "command remains elusive"
        }
        
        It "Shows butler suggestions for missing commands" {
            $Output = & $Script:RbPath exec nonexistent_gem_tool 2>&1
            $LASTEXITCODE | Should -Be 127
            
            ($Output -join " ") | Should -Match "Might I suggest"
            ($Output -join " ") | Should -Match "command name.*spelled correctly"
            ($Output -join " ") | Should -Match "gem install nonexistent_gem_tool"
            ($Output -join " ") | Should -Match "bundle install"
            ($Output -join " ") | Should -Match "diagnostic information.*-v.*-vv"
        }
        
        It "Returns exit code 127 for command not found (Unix convention)" {
            & $Script:RbPath exec definitely_fake_command_xyz 2>&1 | Out-Null
            $LASTEXITCODE | Should -Be 127
        }
        
        It "Displays the exact command name in error message" {
            $TestCommand = "my_custom_missing_tool"
            
            $Output = & $Script:RbPath exec $TestCommand 2>&1
            $LASTEXITCODE | Should -Be 127
            
            ($Output -join " ") | Should -Match "my_custom_missing_tool"
        }
        
        It "Handles commands with arguments gracefully" {
            $Output = & $Script:RbPath exec nonexistent_tool --version --help 2>&1
            $LASTEXITCODE | Should -Be 127
            
            ($Output -join " ") | Should -Match "nonexistent_tool.*appears to be"
            ($Output -join " ") | Should -Match "entirely absent"
        }
    }
    
    Context "Error Message Content Verification" {
        It "Contains all required butler language elements" {
            $Output = & $Script:RbPath exec fake_butler_test_cmd 2>&1
            $LASTEXITCODE | Should -Be 127
            
            $OutputText = $Output -join " "
            
            # Check for sophisticated language
            $OutputText | Should -Match "sincerest apologies"
            $OutputText | Should -Match "humble Butler"
            $OutputText | Should -Match "distinguished Ruby environment"
            $OutputText | Should -Match "meticulously searched"
            $OutputText | Should -Match "remains elusive"
            
            # Check for helpful suggestions
            $OutputText | Should -Match "gem install"
            $OutputText | Should -Match "bundle install"
            $OutputText | Should -Match "spelled correctly"
            $OutputText | Should -Match "diagnostic information"
            
            # Check for debugging hints
            $OutputText | Should -Match "-v.*-vv"
        }
        
        It "Uses distinguished formatting with butler emoji" {
            $Output = & $Script:RbPath exec test_format_cmd 2>&1
            $LASTEXITCODE | Should -Be 127
            
            # Check for butler emoji - handle encoding variations
            ($Output -join " ") | Should -Match "🎩|My sincerest apologies"
        }
        
        It "Provides specific gem install suggestion with command name" {
            $TestCommand = "specific_gem_tool"
            $Output = & $Script:RbPath exec $TestCommand 2>&1
            $LASTEXITCODE | Should -Be 127
            
            ($Output -join " ") | Should -Match "gem install specific_gem_tool"
        }
    }
    
    Context "Different Command Scenarios" {
        It "Handles single character commands" {
            $Output = & $Script:RbPath exec z 2>&1
            $LASTEXITCODE | Should -Be 127
            
            ($Output -join " ") | Should -Match "command 'z' appears to be"
        }
        
        It "Handles commands with special characters" {
            $Output = & $Script:RbPath exec "test-command_123" 2>&1
            $LASTEXITCODE | Should -Be 127
            
            ($Output -join " ") | Should -Match "test-command_123"
        }
        
        It "Handles empty exec command gracefully" {
            $Output = & $Script:RbPath exec 2>&1
            $LASTEXITCODE | Should -Be 1
            
            # This should hit the "No program specified" error, not command not found
            ($Output -join " ") | Should -Match "Request Incomplete.*No program specified"
            ($Output -join " ") | Should -Not -Match "command.*appears to be.*absent"
        }
    }
    
    Context "Interaction with Butler Environment" {
        It "Command not found error appears after butler environment setup" {
            $Output = & $Script:RbPath exec nonexistent_after_setup 2>&1
            $LASTEXITCODE | Should -Be 127
            
            # Should not see bundler preparation messages
            ($Output -join " ") | Should -Not -Match "Butler Notice.*synchronization"
            ($Output -join " ") | Should -Not -Match "meticulously prepared"
            
            # Should see command not found
            ($Output -join " ") | Should -Match "command.*appears to be.*entirely absent"
        }
        
        It "Maintains proper exit code regardless of Ruby environment" {
            # Test with different arguments to ensure consistent behavior
            $TestCommands = @("fake_cmd1", "fake_cmd2", "nonexistent_tool")
            
            foreach ($Command in $TestCommands) {
                & $Script:RbPath exec $Command 2>&1 | Out-Null
                $LASTEXITCODE | Should -Be 127
            }
        }
    }
}