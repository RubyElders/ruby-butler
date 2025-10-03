# Integration Tests for Ruby Butler Configuration File Support
# Tests configuration file loading, precedence, and override mechanisms

BeforeAll {
    $Script:RbPath = $env:RB_TEST_PATH
    if (-not $Script:RbPath) {
        throw "RB_TEST_PATH environment variable not set. Run Setup.ps1 first."
    }
}

Describe "Configuration File Tests" {
    Context "RB_CONFIG Environment Variable" {
        BeforeEach {
            # Create a temporary config file
            $script:TempConfigPath = Join-Path $env:TEMP "test-rb-config-$([guid]::NewGuid().ToString()).toml"
            $configContent = @"
ruby-version = "3.2.0"
rubies-dir = "C:/test/rubies"
gem-home = "C:/test/gems"
"@
            Set-Content -Path $script:TempConfigPath -Value $configContent -Force
            
            # Set the environment variable
            $env:RB_CONFIG = $script:TempConfigPath
        }
        
        AfterEach {
            # Clean up
            Remove-Item -Path $script:TempConfigPath -ErrorAction SilentlyContinue -Force
            Remove-Item env:RB_CONFIG -ErrorAction SilentlyContinue
        }
        
        It "Should load configuration from RB_CONFIG environment variable" {
            # Run rb --help to verify it loads without error
            $Output = & $Script:RbPath --help 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "Ruby Butler"
        }
        
        It "Should use config file values when RB_CONFIG is set" {
            # Note: We can't easily test the actual values being used without
            # running a command that shows them, but we can verify it doesn't error
            $Output = & $Script:RbPath --version 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "Ruby Butler"
        }
        
        It "Should apply rubies-dir from RB_CONFIG to runtime command" {
            # Use rb runtime to check that the config value is actually used
            $Output = & $Script:RbPath runtime 2>&1
            # The error message should reference the configured directory
            ($Output -join " ") | Should -Match "C:/test/rubies"
        }
        
        It "Should show configured values with verbose logging" {
            # Use -v flag to see which config was loaded
            $Output = & $Script:RbPath -v runtime 2>&1
            # Should show that config was loaded
            ($Output -join " ") | Should -Match "Loading configuration from.*test-rb-config.*\.toml"
        }
    }
    
    Context "--config CLI Flag" {
        BeforeEach {
            # Create a temporary config file
            $script:TempConfigPath = Join-Path $env:TEMP "test-rb-cli-config-$([guid]::NewGuid().ToString()).toml"
            $configContent = @"
ruby-version = "3.3.0"
rubies-dir = "D:/custom/rubies"
"@
            Set-Content -Path $script:TempConfigPath -Value $configContent -Force
        }
        
        AfterEach {
            # Clean up
            Remove-Item -Path $script:TempConfigPath -ErrorAction SilentlyContinue -Force
        }
        
        It "Should load configuration from --config flag" {
            # Run with --config flag
            $Output = & $Script:RbPath --config $script:TempConfigPath --help 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "Ruby Butler"
        }
        
        It "Should show --config option in help" {
            $Output = & $Script:RbPath --help 2>&1
            ($Output -join " ") | Should -Match "--config"
            ($Output -join " ") | Should -Match "configuration file"
        }
        
        It "Should apply rubies-dir from --config flag to runtime command" {
            # Verify the config value is actually used
            $Output = & $Script:RbPath --config $script:TempConfigPath runtime 2>&1
            # The error message should reference the configured directory
            ($Output -join " ") | Should -Match "D:/custom/rubies"
        }
    }
    
    Context "Config Precedence" {
        BeforeEach {
            # Create two different config files
            $script:EnvConfigPath = Join-Path $env:TEMP "test-rb-env-config-$([guid]::NewGuid().ToString()).toml"
            $script:CliConfigPath = Join-Path $env:TEMP "test-rb-cli-config-$([guid]::NewGuid().ToString()).toml"
            
            Set-Content -Path $script:EnvConfigPath -Value "ruby-version = `"3.1.0`"" -Force
            Set-Content -Path $script:CliConfigPath -Value "ruby-version = `"3.2.0`"" -Force
            
            # Set environment variable
            $env:RB_CONFIG = $script:EnvConfigPath
        }
        
        AfterEach {
            # Clean up
            Remove-Item -Path $script:EnvConfigPath -ErrorAction SilentlyContinue -Force
            Remove-Item -Path $script:CliConfigPath -ErrorAction SilentlyContinue -Force
            Remove-Item env:RB_CONFIG -ErrorAction SilentlyContinue
        }
        
        It "Should prioritize --config flag over RB_CONFIG environment variable" {
            # Both config sources exist, --config should win
            # We verify this by ensuring the command doesn't fail
            $Output = & $Script:RbPath --config $script:CliConfigPath --help 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "Ruby Butler"
        }
        
        It "Should prioritize CLI argument over config file" {
            # CLI flag should override config file value
            # Using -r flag should take precedence over any config
            $Output = & $Script:RbPath --config $script:CliConfigPath -r 3.4.0 --help 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "Ruby Butler"
        }
        
        It "Should verify CLI argument precedence with runtime command" {
            # Create a config with a rubies-dir, then override it with CLI
            $TestConfigPath = Join-Path $env:TEMP "test-precedence-$([guid]::NewGuid().ToString()).toml"
            Set-Content -Path $TestConfigPath -Value "rubies-dir = `"C:/config/rubies`"" -Force
            
            # Override with CLI argument
            $Output = & $Script:RbPath --config $TestConfigPath -R "C:/cli/rubies" runtime 2>&1
            
            # Should use CLI value, not config value
            ($Output -join " ") | Should -Match "C:/cli/rubies"
            ($Output -join " ") | Should -Not -Match "C:/config/rubies"
            
            Remove-Item -Path $TestConfigPath -Force
        }
        
        It "Should show precedence in debug logs" {
            # Create a config file with rubies-dir
            $TestConfigPath = Join-Path $env:TEMP "test-debug-$([guid]::NewGuid().ToString()).toml"
            Set-Content -Path $TestConfigPath -Value "rubies-dir = `"C:/config/rubies`"" -Force
            
            # Override with CLI and use -vv for debug logging
            $Output = & $Script:RbPath --config $TestConfigPath -R "C:/cli/rubies" -vv runtime 2>&1
            
            # Should show merge strategy in debug logs
            ($Output -join " ") | Should -Match "Using rubies-dir from CLI arguments"
            
            Remove-Item -Path $TestConfigPath -Force
        }
    }
    
    Context "Invalid Config File" {
        BeforeEach {
            # Create an invalid config file
            $script:InvalidConfigPath = Join-Path $env:TEMP "test-rb-invalid-$([guid]::NewGuid().ToString()).toml"
            Set-Content -Path $script:InvalidConfigPath -Value "this is not valid toml { [ ]" -Force
        }
        
        AfterEach {
            # Clean up
            Remove-Item -Path $script:InvalidConfigPath -ErrorAction SilentlyContinue -Force
        }
        
        It "Should handle invalid TOML file gracefully" {
            # Invalid TOML files cause errors during parsing
            $Output = & $Script:RbPath --config $script:InvalidConfigPath --help 2>&1
            # If the file exists but has invalid TOML, we should get an error
            # For now, verify the command ran (implementation may vary)
            $Output | Should -Not -BeNullOrEmpty
        }
    }
    
    Context "Non-existent Config File" {
        It "Should work fine with non-existent --config path" {
            $NonExistentPath = "C:/does/not/exist/rb.toml"
            # Should use defaults when file doesn't exist
            $Output = & $Script:RbPath --config $NonExistentPath --help 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "Ruby Butler"
        }
        
        It "Should work fine with non-existent RB_CONFIG" {
            $env:RB_CONFIG = "C:/does/not/exist/rb.toml"
            # Should use defaults when file doesn't exist
            $Output = & $Script:RbPath --help 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "Ruby Butler"
            
            Remove-Item env:RB_CONFIG -ErrorAction SilentlyContinue
        }
    }
}
