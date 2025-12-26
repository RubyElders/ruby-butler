# Integration Tests for Ruby Butler Directory Not Found Error Handling
# Tests error handling when rubies directory doesn't exist

BeforeAll {
    $Script:RbPath = $env:RB_TEST_PATH
    if (-not $Script:RbPath) {
        throw "RB_TEST_PATH environment variable not set. Run Setup.ps1 first."
    }
}

Describe "Ruby Butler - Directory Not Found Error Handling" {
    Context "Nonexistent Directory Error Messages" {
        It "Shows error message for relative path" {
            $NonexistentDir = "completely_nonexistent_test_directory_12345"
            
            $Output = & $Script:RbPath -R $NonexistentDir rt 2>&1
            $LASTEXITCODE | Should -Be 1
            
            ($Output -join " ") | Should -Match "Ruby installation directory not found"
            ($Output -join " ") | Should -Match $NonexistentDir
        }
        
        It "Shows error message for absolute path" {
            $NonexistentDir = "C:\completely_nonexistent_test_directory_12345"
            
            $Output = & $Script:RbPath -R $NonexistentDir environment 2>&1
            $LASTEXITCODE | Should -Be 1
            
            ($Output -join " ") | Should -Match "Ruby installation directory not found"
            ($Output -join " ") | Should -Match "completely_nonexistent_test_directory_12345"
        }
        
        It "Shows directory path clearly in error message" {
            $TestDir = "my_custom_rubies_dir"
            
            $Output = & $Script:RbPath -R $TestDir exec ruby --version 2>&1
            $LASTEXITCODE | Should -Be 1
            
            ($Output -join " ") | Should -Match "my_custom_rubies_dir"
        }
        
        It "Returns exit code 1 for directory not found" {
            & $Script:RbPath -R "nonexistent_exit_code_test" rt 2>&1 | Out-Null
            $LASTEXITCODE | Should -Be 1
        }
        
        It "Maintains consistent error across different commands" {
            $TestCommands = @("runtime", "rt", "environment", "env")
            
            foreach ($Command in $TestCommands) {
                $Output = & $Script:RbPath -R "nonexistent_$Command" $Command 2>&1
                $LASTEXITCODE | Should -Be 1
                ($Output -join " ") | Should -Match "Ruby installation directory not found"
            }
        }
    }
    
    Context "Error Message Content Verification" {
        It "Contains helpful guidance" {
            $Output = & $Script:RbPath -R "test_content_dir" rt 2>&1
            $LASTEXITCODE | Should -Be 1
            
            $OutputText = $Output -join " "
            
            # Check for helpful guidance
            $OutputText | Should -Match "Ruby installation directory not found"
            $OutputText | Should -Match "verify the path exists"
            $OutputText | Should -Match "RB_RUBIES_DIR"
        }
        
        It "Displays the exact directory path provided" {
            $CustomPath = "my_custom_ruby_path"
            $Output = & $Script:RbPath -R $CustomPath rt 2>&1
            $LASTEXITCODE | Should -Be 1
            
            ($Output -join " ") | Should -Match "my_custom_ruby_path"
        }
    }
}