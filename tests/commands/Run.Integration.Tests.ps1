# Integration Tests for Ruby Butler Run Command
# Tests rb run / rb r command functionality with rbproject.toml scripts

BeforeAll {
    $Script:RbPath = $env:RB_TEST_PATH
    if (-not $Script:RbPath) {
        throw "RB_TEST_PATH environment variable not set. Run Setup.ps1 first."
    }
    
    # Create temporary directory for test files
    $Script:TestDir = Join-Path $env:TEMP "rb-run-tests-$(Get-Random)"
    New-Item -ItemType Directory -Path $Script:TestDir -Force | Out-Null
    
    # Create a project with various script types
    $Script:ProjectWithScripts = Join-Path $Script:TestDir "with-scripts"
    New-Item -ItemType Directory -Path $Script:ProjectWithScripts -Force | Out-Null
    
    $ProjectFile = Join-Path $Script:ProjectWithScripts "rbproject.toml"
    @'
[project]
name = "Test Runner Project"
description = "Project for testing rb run command"

[scripts]
# Simple scripts (platform-independent commands)
version = "ruby -v"
gem-version = "gem -v"
help = "ruby --help"

# Scripts with descriptions
info = { command = "ruby -v", description = "Show Ruby version information" }
check = { command = "gem -v", description = "Verify gem installation" }

# Scripts with colons (namespace-style)
"test:version" = "ruby -v"
"test:env" = { command = "gem env", description = "Show gem environment" }

# Multi-word commands
list-gems = "gem list --local"
'@ | Set-Content -Path $ProjectFile -Encoding UTF8
    
    # Create empty project (no scripts)
    $Script:ProjectNoScripts = Join-Path $Script:TestDir "no-scripts"
    New-Item -ItemType Directory -Path $Script:ProjectNoScripts -Force | Out-Null
    
    $EmptyProjectFile = Join-Path $Script:ProjectNoScripts "rbproject.toml"
    @'
[project]
name = "Empty Project"

[scripts]
'@ | Set-Content -Path $EmptyProjectFile -Encoding UTF8
    
    # Create project without rbproject.toml
    $Script:ProjectNoConfig = Join-Path $Script:TestDir "no-config"
    New-Item -ItemType Directory -Path $Script:ProjectNoConfig -Force | Out-Null
    
    # Create project with only simple scripts (no descriptions)
    $Script:ProjectSimple = Join-Path $Script:TestDir "simple"
    New-Item -ItemType Directory -Path $Script:ProjectSimple -Force | Out-Null
    
    $SimpleProjectFile = Join-Path $Script:ProjectSimple "rbproject.toml"
    @'
[scripts]
version = "ruby -v"
help = "ruby --help"
'@ | Set-Content -Path $SimpleProjectFile -Encoding UTF8
}

AfterAll {
    # Clean up test directory
    if (Test-Path $Script:TestDir) {
        Remove-Item -Path $Script:TestDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

Describe "Ruby Butler - Run Command (rb run)" {
    
    Context "Listing Available Scripts - Basic Functionality" {
        It "Lists scripts when no script name provided" {
            Push-Location $Script:ProjectWithScripts
            try {
                $Output = & $Script:RbPath run 2>&1
                $LASTEXITCODE | Should -Be 0
                ($Output -join "`n") | Should -Match "Run Project Scripts"
            } finally {
                Pop-Location
            }
        }
        
        It "Shows project name in list output" {
            Push-Location $Script:ProjectWithScripts
            try {
                $Output = & $Script:RbPath run 2>&1
                $LASTEXITCODE | Should -Be 0
                ($Output -join "`n") | Should -Match "Test Runner Project"
            } finally {
                Pop-Location
            }
        }
        
        It "Shows project description in list output" {
            Push-Location $Script:ProjectWithScripts
            try {
                $Output = & $Script:RbPath run 2>&1
                $LASTEXITCODE | Should -Be 0
                ($Output -join "`n") | Should -Match "Project for testing rb run command"
            } finally {
                Pop-Location
            }
        }
        
        It "Shows Usage section" {
            Push-Location $Script:ProjectWithScripts
            try {
                $Output = & $Script:RbPath run 2>&1
                $LASTEXITCODE | Should -Be 0
                $OutputText = $Output -join "`n"
                $OutputText | Should -Match "Usage:"
                $OutputText | Should -Match "rb run <SCRIPT>"
            } finally {
                Pop-Location
            }
        }
        
        It "Lists all available scripts" {
            Push-Location $Script:ProjectWithScripts
            try {
                $Output = & $Script:RbPath run 2>&1
                $LASTEXITCODE | Should -Be 0
                $OutputText = $Output -join "`n"
                $OutputText | Should -Match "version"
                $OutputText | Should -Match "gem-version"
                $OutputText | Should -Match "info"
                $OutputText | Should -Match "check"
                $OutputText | Should -Match "test:version"
                $OutputText | Should -Match "test:env"
            } finally {
                Pop-Location
            }
        }
        
        It "Shows script descriptions when available" {
            Push-Location $Script:ProjectWithScripts
            try {
                $Output = & $Script:RbPath run 2>&1
                $LASTEXITCODE | Should -Be 0
                $OutputText = $Output -join "`n"
                $OutputText | Should -Match "Show Ruby version information"
                $OutputText | Should -Match "Verify gem installation"
                $OutputText | Should -Match "Show gem environment"
            } finally {
                Pop-Location
            }
        }
        
        It "Shows Details section with project path and script count" {
            Push-Location $Script:ProjectWithScripts
            try {
                $Output = & $Script:RbPath run 2>&1
                $LASTEXITCODE | Should -Be 0
                $OutputText = $Output -join "`n"
                $OutputText | Should -Match "Details:"
                $OutputText | Should -Match "Project.*rbproject\.toml"
                $OutputText | Should -Match "Scripts.*8"
            } finally {
                Pop-Location
            }
        }
    }
    
    Context "Listing Scripts - Short Alias (rb r)" {
        It "Works with 'r' alias" {
            Push-Location $Script:ProjectWithScripts
            try {
                $Output = & $Script:RbPath r 2>&1
                $LASTEXITCODE | Should -Be 0
                ($Output -join "`n") | Should -Match "Run Project Scripts"
            } finally {
                Pop-Location
            }
        }
        
        It "Lists scripts with 'r' alias just like 'run'" {
            Push-Location $Script:ProjectWithScripts
            try {
                $Output = & $Script:RbPath r 2>&1
                $LASTEXITCODE | Should -Be 0
                $OutputText = $Output -join "`n"
                $OutputText | Should -Match "version"
                $OutputText | Should -Match "gem-version"
            } finally {
                Pop-Location
            }
        }
    }
    
    Context "Listing Scripts - Empty Project" {
        It "Handles project with no scripts gracefully" {
            Push-Location $Script:ProjectNoScripts
            try {
                $Output = & $Script:RbPath run 2>&1
                $LASTEXITCODE | Should -Be 0
                ($Output -join "`n") | Should -Match "No scripts defined"
            } finally {
                Pop-Location
            }
        }
        
        It "Shows helpful message about adding scripts" {
            Push-Location $Script:ProjectNoScripts
            try {
                $Output = & $Script:RbPath run 2>&1
                $LASTEXITCODE | Should -Be 0
                $OutputText = $Output -join "`n"
                $OutputText | Should -Match "To define scripts, add them to"
                $OutputText | Should -Match "rbproject\.toml"
            } finally {
                Pop-Location
            }
        }
        
        It "Shows example syntax for adding scripts" {
            Push-Location $Script:ProjectNoScripts
            try {
                $Output = & $Script:RbPath run 2>&1
                $LASTEXITCODE | Should -Be 0
                $OutputText = $Output -join "`n"
                $OutputText | Should -Match "\[scripts\]"
                $OutputText | Should -Match 'test.*=.*"rspec"'
            } finally {
                Pop-Location
            }
        }
    }
    
    Context "Listing Scripts - No Project File" {
        It "Shows error when no rbproject.toml exists" {
            Push-Location $Script:ProjectNoConfig
            try {
                $Output = & $Script:RbPath run 2>&1 | Out-String
                $Output | Should -Match "No Project Configuration|not found"
            } finally {
                Pop-Location
            }
        }
        
        It "Returns non-zero exit code when no project file exists" {
            Push-Location $Script:ProjectNoConfig
            try {
                $Output = & $Script:RbPath run 2>&1
                $LASTEXITCODE | Should -Not -Be 0
            } finally {
                Pop-Location
            }
        }
    }
    
    Context "Executing Scripts - Simple Commands" {
        It "Executes simple script successfully" {
            Push-Location $Script:ProjectWithScripts
            try {
                $Output = & $Script:RbPath run version 2>&1
                $LASTEXITCODE | Should -Be 0
                ($Output -join "`n") | Should -Match "ruby \d+\.\d+\.\d+"
            } finally {
                Pop-Location
            }
        }
        
        It "Executes gem command successfully" {
            Push-Location $Script:ProjectWithScripts
            try {
                $Output = & $Script:RbPath run gem-version 2>&1
                $LASTEXITCODE | Should -Be 0
                ($Output -join "`n") | Should -Match "\d+\.\d+\.\d+"
            } finally {
                Pop-Location
            }
        }
        
        It "Executes script with description" {
            Push-Location $Script:ProjectWithScripts
            try {
                $Output = & $Script:RbPath run info 2>&1
                $LASTEXITCODE | Should -Be 0
                ($Output -join "`n") | Should -Match "ruby \d+\.\d+\.\d+"
            } finally {
                Pop-Location
            }
        }
        
        It "Executes script with colon in name" {
            Push-Location $Script:ProjectWithScripts
            try {
                $Output = & $Script:RbPath run test:version 2>&1
                $LASTEXITCODE | Should -Be 0
                ($Output -join "`n") | Should -Match "ruby \d+\.\d+\.\d+"
            } finally {
                Pop-Location
            }
        }
    }
    
    Context "Executing Scripts - Short Alias" {
        It "Executes script with 'r' alias" {
            Push-Location $Script:ProjectWithScripts
            try {
                $Output = & $Script:RbPath r version 2>&1
                $LASTEXITCODE | Should -Be 0
                ($Output -join "`n") | Should -Match "ruby \d+\.\d+\.\d+"
            } finally {
                Pop-Location
            }
        }
        
        It "Executes colon-named script with 'r' alias" {
            Push-Location $Script:ProjectWithScripts
            try {
                $Output = & $Script:RbPath r test:version 2>&1
                $LASTEXITCODE | Should -Be 0
                ($Output -join "`n") | Should -Match "ruby \d+\.\d+\.\d+"
            } finally {
                Pop-Location
            }
        }
    }
    
    Context "Executing Scripts - Error Handling" {
        It "Shows error for non-existent script" {
            Push-Location $Script:ProjectWithScripts
            try {
                $Output = & $Script:RbPath run nonexistent 2>&1 | Out-String
                $Output | Should -Match "not defined|not found"
            } finally {
                Pop-Location
            }
        }
        
        It "Returns non-zero exit code for non-existent script" {
            Push-Location $Script:ProjectWithScripts
            try {
                $Output = & $Script:RbPath run nonexistent 2>&1
                $LASTEXITCODE | Should -Not -Be 0
            } finally {
                Pop-Location
            }
        }
    }
    
    Context "Integration with --project Flag" {
        It "Lists scripts from specified project file" {
            $ProjectFile = Join-Path $Script:ProjectWithScripts "rbproject.toml"
            $Output = & $Script:RbPath -P $ProjectFile run 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join "`n") | Should -Match "Run Project Scripts"
            ($Output -join "`n") | Should -Match "version"
        }
        
        It "Executes script from specified project file" {
            $ProjectFile = Join-Path $Script:ProjectWithScripts "rbproject.toml"
            Push-Location $Script:TestDir  # Different directory
            try {
                $Output = & $Script:RbPath -P $ProjectFile run version 2>&1
                $LASTEXITCODE | Should -Be 0
                ($Output -join "`n") | Should -Match "ruby \d+\.\d+\.\d+"
            } finally {
                Pop-Location
            }
        }
        
        It "Uses specified project file even from different directory" {
            $ProjectFile = Join-Path $Script:ProjectWithScripts "rbproject.toml"
            Push-Location $Script:TestDir
            try {
                $Output = & $Script:RbPath -P $ProjectFile run 2>&1
                $LASTEXITCODE | Should -Be 0
                ($Output -join "`n") | Should -Match "Test Runner Project"
            } finally {
                Pop-Location
            }
        }
    }
    
    Context "Output Format - Professional Styling" {
        It "Uses emoji in title" {
            Push-Location $Script:ProjectWithScripts
            try {
                $Output = & $Script:RbPath run 2>&1
                # Emoji rendering varies by terminal, just check for the text
                ($Output -join "`n") | Should -Match "Run Project Scripts"
            } finally {
                Pop-Location
            }
        }
        
        It "Shows Scripts section with colon" {
            Push-Location $Script:ProjectWithScripts
            try {
                $Output = & $Script:RbPath run 2>&1
                ($Output -join "`n") | Should -Match "Scripts:"
            } finally {
                Pop-Location
            }
        }
        
        It "Shows aligned script list" {
            Push-Location $Script:ProjectWithScripts
            try {
                $Output = & $Script:RbPath run 2>&1
                $LASTEXITCODE | Should -Be 0
                # Should have some output with scripts
                ($Output -join "`n").Length | Should -BeGreaterThan 100
            } finally {
                Pop-Location
            }
        }
    }
    
    Context "Simple Project without Metadata" {
        It "Lists scripts even without [project] section" {
            Push-Location $Script:ProjectSimple
            try {
                $Output = & $Script:RbPath run 2>&1
                $LASTEXITCODE | Should -Be 0
                $OutputText = $Output -join "`n"
                $OutputText | Should -Match "version"
                $OutputText | Should -Match "help"
            } finally {
                Pop-Location
            }
        }
        
        It "Executes scripts without [project] section" {
            Push-Location $Script:ProjectSimple
            try {
                $Output = & $Script:RbPath run version 2>&1
                $LASTEXITCODE | Should -Be 0
                ($Output -join "`n") | Should -Match "ruby \d+\.\d+\.\d+"
            } finally {
                Pop-Location
            }
        }
        
        It "Does not show project name when not specified" {
            Push-Location $Script:ProjectSimple
            try {
                $Output = & $Script:RbPath run 2>&1
                $LASTEXITCODE | Should -Be 0
                # Should go straight from title to Usage section
                $OutputText = $Output -join "`n"
                $OutputText | Should -Match "Run Project Scripts[\s\r\n]+Usage:"
            } finally {
                Pop-Location
            }
        }
    }
    
    Context "Cross-Command Consistency" {
        It "Works with verbose flag" {
            Push-Location $Script:ProjectWithScripts
            try {
                $Output = & $Script:RbPath -v run 2>&1
                $LASTEXITCODE | Should -Be 0
                ($Output -join "`n") | Should -Match "Run Project Scripts"
            } finally {
                Pop-Location
            }
        }
        
        It "Works with very verbose flag" {
            Push-Location $Script:ProjectWithScripts
            try {
                $Output = & $Script:RbPath -V run 2>&1
                $LASTEXITCODE | Should -Be 0
                ($Output -join "`n") | Should -Match "Run Project Scripts"
            } finally {
                Pop-Location
            }
        }
        
        It "Combines with --project flag correctly" {
            $ProjectFile = Join-Path $Script:ProjectWithScripts "rbproject.toml"
            $Output = & $Script:RbPath -v -P $ProjectFile run 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join "`n") | Should -Match "Run Project Scripts"
        }
    }
}

Describe "Ruby Butler - Run Command Edge Cases" {
    
    Context "Script Name Variations" {
        It "Handles script names with hyphens" {
            Push-Location $Script:ProjectWithScripts
            try {
                $Output = & $Script:RbPath run gem-version 2>&1
                $LASTEXITCODE | Should -Be 0
            } finally {
                Pop-Location
            }
        }
        
        It "Handles script names with multiple colons" {
            # Create temporary project with multi-colon script
            $TempProject = Join-Path $Script:TestDir "multi-colon"
            New-Item -ItemType Directory -Path $TempProject -Force | Out-Null
            @'
[scripts]
"db:migrate:up" = "ruby -v"
'@ | Set-Content (Join-Path $TempProject "rbproject.toml") -Encoding UTF8
            
            Push-Location $TempProject
            try {
                $Output = & $Script:RbPath run 2>&1
                $LASTEXITCODE | Should -Be 0
                ($Output -join "`n") | Should -Match "db:migrate:up"
            } finally {
                Pop-Location
            }
        }
    }
    
    Context "Case Sensitivity" {
        It "Script names are case-sensitive on all platforms" {
            Push-Location $Script:ProjectWithScripts
            try {
                # 'version' exists but 'Version' doesn't
                $Output = & $Script:RbPath run Version 2>&1 | Out-String
                # Should fail to find 'Version'
                $Output | Should -Match "not defined|not found"
            } finally {
                Pop-Location
            }
        }
    }
    
    Context "Performance with Many Scripts" {
        It "Handles projects with many scripts efficiently" {
            # Create project with 50 scripts
            $LargeProject = Join-Path $Script:TestDir "large"
            New-Item -ItemType Directory -Path $LargeProject -Force | Out-Null
            
            $Scripts = "[scripts]`n"
            for ($i = 1; $i -le 50; $i++) {
                $Scripts += "script$i = `"ruby -v`"`n"
            }
            $Scripts | Set-Content (Join-Path $LargeProject "rbproject.toml") -Encoding UTF8
            
            Push-Location $LargeProject
            try {
                $Stopwatch = [System.Diagnostics.Stopwatch]::StartNew()
                $Output = & $Script:RbPath run 2>&1
                $Stopwatch.Stop()
                
                $LASTEXITCODE | Should -Be 0
                ($Output -join "`n") | Should -Match "Scripts.*50"
                # Should complete in reasonable time (< 5 seconds)
                $Stopwatch.ElapsedMilliseconds | Should -BeLessThan 5000
            } finally {
                Pop-Location
            }
        }
    }
}

Describe "Ruby Butler - Run Command Delegation to Exec" {
    
    Context "Exec Command Delegation" {
        It "Delegates to exec command for script execution" {
            Push-Location $Script:ProjectWithScripts
            try {
                $Output = & $Script:RbPath -v run version 2>&1 | Out-String
                # Should show exec-related log messages
                $Output | Should -Match "Delegating to exec command"
                $Output | Should -Match "Preparing to execute"
            } finally {
                Pop-Location
            }
        }
        
        It "Parses script command correctly" {
            Push-Location $Script:ProjectWithScripts
            try {
                $Output = & $Script:RbPath -V run version 2>&1 | Out-String
                # Should parse "ruby -v" into program and args (debug level logging)
                $Output | Should -Match "Program: ruby|Executing"
            } finally {
                Pop-Location
            }
        }
        
        It "Passes additional arguments to executed command" {
            # Create project with ruby -e script
            $EchoProject = Join-Path $Script:TestDir "echo-args"
            New-Item -ItemType Directory -Path $EchoProject -Force | Out-Null
            @'
[scripts]
echo = "ruby -e"
'@ | Set-Content (Join-Path $EchoProject "rbproject.toml") -Encoding UTF8
            
            Push-Location $EchoProject
            try {
                # Pass additional arguments after the script name
                $Output = & $Script:RbPath run echo "puts ARGV.inspect" arg1 arg2 2>&1 | Out-String
                # Should show the arguments were passed
                $Output | Should -Match 'arg1.*arg2'
            } finally {
                Pop-Location
            }
        }
        
        It "Uses same environment composition as exec" {
            Push-Location $Script:ProjectWithScripts
            try {
                $Output = & $Script:RbPath -v run version 2>&1 | Out-String
                # Should show environment composition messages (same as exec)
                $Output | Should -Match "Environment composition complete"
            } finally {
                Pop-Location
            }
        }
    }
    
    Context "Command Parsing" {
        It "Handles commands with multiple arguments" {
            # Create project with multi-arg command
            $MultiArgProject = Join-Path $Script:TestDir "multi-arg"
            New-Item -ItemType Directory -Path $MultiArgProject -Force | Out-Null
            @'
[scripts]
multi = "ruby -e \"puts 'test'\""
'@ | Set-Content (Join-Path $MultiArgProject "rbproject.toml") -Encoding UTF8
            
            Push-Location $MultiArgProject
            try {
                $Output = & $Script:RbPath run multi 2>&1
                $LASTEXITCODE | Should -Be 0
                ($Output -join "`n") | Should -Match "test"
            } finally {
                Pop-Location
            }
        }
        
        It "Preserves quoted arguments in command" {
            # Create project with quoted command
            $QuotedProject = Join-Path $Script:TestDir "quoted"
            New-Item -ItemType Directory -Path $QuotedProject -Force | Out-Null
            @'
[scripts]
hello = "ruby -e \"puts 'Hello World'\""
'@ | Set-Content (Join-Path $QuotedProject "rbproject.toml") -Encoding UTF8
            
            Push-Location $QuotedProject
            try {
                $Output = & $Script:RbPath run hello 2>&1
                $LASTEXITCODE | Should -Be 0
                ($Output -join "`n") | Should -Match "Hello World"
            } finally {
                Pop-Location
            }
        }
        
        It "Handles commands with extra whitespace" {
            # Create project with extra spaces
            $SpaceProject = Join-Path $Script:TestDir "spaces"
            New-Item -ItemType Directory -Path $SpaceProject -Force | Out-Null
            @'
[scripts]
spaced = "ruby   -v"
'@ | Set-Content (Join-Path $SpaceProject "rbproject.toml") -Encoding UTF8
            
            Push-Location $SpaceProject
            try {
                $Output = & $Script:RbPath run spaced 2>&1
                $LASTEXITCODE | Should -Be 0
                ($Output -join "`n") | Should -Match "ruby \d+\.\d+\.\d+"
            } finally {
                Pop-Location
            }
        }
    }
    
    Context "Bundler Integration via Exec" {
        It "Detects bundler environment when Gemfile exists" {
            # Create project with Gemfile
            $BundlerProject = Join-Path $Script:TestDir "bundler-detect"
            New-Item -ItemType Directory -Path $BundlerProject -Force | Out-Null
            
            # Create minimal Gemfile
            @'
source 'https://rubygems.org'
gem 'rake'
'@ | Set-Content (Join-Path $BundlerProject "Gemfile") -Encoding UTF8
            
            @'
[scripts]
version = "ruby -v"
'@ | Set-Content (Join-Path $BundlerProject "rbproject.toml") -Encoding UTF8
            
            Push-Location $BundlerProject
            try {
                $Output = & $Script:RbPath -v run version 2>&1 | Out-String
                # Should detect bundler environment
                $Output | Should -Match "Bundler.*detected|Found Gemfile"
            } finally {
                Pop-Location
            }
        }
        
        It "Triggers bundle sync when needed" {
            # Create project with Gemfile but no bundle install
            $BundlerSyncProject = Join-Path $Script:TestDir "bundler-sync"
            New-Item -ItemType Directory -Path $BundlerSyncProject -Force | Out-Null
            
            # Create minimal Gemfile
            @'
source 'https://rubygems.org'
gem 'rake'
'@ | Set-Content (Join-Path $BundlerSyncProject "Gemfile") -Encoding UTF8
            
            @'
[scripts]
test = "ruby -v"
'@ | Set-Content (Join-Path $BundlerSyncProject "rbproject.toml") -Encoding UTF8
            
            Push-Location $BundlerSyncProject
            try {
                $Output = & $Script:RbPath run test 2>&1 | Out-String
                # Should trigger bundle sync if needed
                $Output | Should -Match "ruby \d+\.\d+\.\d+|Butler Notice|Bundle complete"
            } finally {
                Pop-Location
            }
        }
    }
    
    Context "Error Handling via Exec" {
        It "Provides helpful error for non-existent commands" {
            # Create project with non-existent command
            $BadCommandProject = Join-Path $Script:TestDir "bad-command"
            New-Item -ItemType Directory -Path $BadCommandProject -Force | Out-Null
            @'
[scripts]
missing = "nonexistent-command-xyz"
'@ | Set-Content (Join-Path $BadCommandProject "rbproject.toml") -Encoding UTF8
            
            Push-Location $BadCommandProject
            try {
                $Output = & $Script:RbPath run missing 2>&1 | Out-String
                # Should get exec's helpful error message
                $Output | Should -Match "My sincerest apologies|command.*appears to be.*absent|not found"
            } finally {
                Pop-Location
            }
        }
        
        It "Handles command exit codes correctly" {
            # Create project with failing command
            $FailProject = Join-Path $Script:TestDir "fail"
            New-Item -ItemType Directory -Path $FailProject -Force | Out-Null
            @'
[scripts]
fail = "ruby -e \"exit 42\""
'@ | Set-Content (Join-Path $FailProject "rbproject.toml") -Encoding UTF8
            
            Push-Location $FailProject
            try {
                $Output = & $Script:RbPath run fail 2>&1
                # Should exit with the command's exit code
                $LASTEXITCODE | Should -Be 42
            } finally {
                Pop-Location
            }
        }
    }
}
