# Integration Tests for Ruby Butler Project Commands
# Tests --project/-P flag functionality with rbproject.toml files

BeforeAll {
    $Script:RbPath = $env:RB_TEST_PATH
    if (-not $Script:RbPath) {
        throw "RB_TEST_PATH environment variable not set. Run Setup.ps1 first."
    }
    
    # Create temporary directory for test files
    $Script:TestDir = Join-Path $env:TEMP "rb-project-tests-$(Get-Random)"
    New-Item -ItemType Directory -Path $Script:TestDir -Force | Out-Null
    
    # Create a valid test rbproject.toml
    $Script:ValidProjectFile = Join-Path $Script:TestDir "valid-project.toml"
    @'
[project]
name = "Test Project"
description = "A test project for Pester testing"

[scripts]
test = "rspec"
"test:watch" = { command = "guard", description = "Watch and run tests" }
lint = { command = "rubocop", description = "Run linter" }
"lint:fix" = "rubocop -a"
'@ | Set-Content -Path $Script:ValidProjectFile -Encoding UTF8
    
    # Create a project file without metadata
    $Script:NoMetadataProjectFile = Join-Path $Script:TestDir "no-metadata.toml"
    @'
[scripts]
test = "rspec"
build = "rake build"
'@ | Set-Content -Path $Script:NoMetadataProjectFile -Encoding UTF8
    
    # Create a project file with only name
    $Script:PartialMetadataProjectFile = Join-Path $Script:TestDir "partial-metadata.toml"
    @'
[project]
name = "Partial Metadata Project"

[scripts]
server = "rails server"
'@ | Set-Content -Path $Script:PartialMetadataProjectFile -Encoding UTF8
    
    # Create an invalid TOML file
    $Script:InvalidTomlFile = Join-Path $Script:TestDir "invalid.toml"
    @'
[project
name = "Invalid TOML - missing closing bracket"

[scripts]
test = "rspec"
'@ | Set-Content -Path $Script:InvalidTomlFile -Encoding UTF8
    
    # Create an empty file
    $Script:EmptyFile = Join-Path $Script:TestDir "empty.toml"
    "" | Set-Content -Path $Script:EmptyFile -Encoding UTF8
    
    # Path for non-existent file
    $Script:NonExistentFile = Join-Path $Script:TestDir "does-not-exist.toml"
}

AfterAll {
    # Clean up test directory
    if (Test-Path $Script:TestDir) {
        Remove-Item -Path $Script:TestDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

Describe "Ruby Butler - Project Flag (-P/--project)" {
    
    Context "Valid Project File with Full Metadata" {
        It "Loads project file specified with -P flag" {
            $Output = & $Script:RbPath -P $Script:ValidProjectFile env 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join "`n") | Should -Match "Project"
        }
        
        It "Loads project file specified with --project flag" {
            $Output = & $Script:RbPath --project $Script:ValidProjectFile env 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join "`n") | Should -Match "Project"
        }
        
        It "Displays project name when specified" {
            $Output = & $Script:RbPath -P $Script:ValidProjectFile env 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join "`n") | Should -Match "Name\s*:\s*Test Project"
        }
        
        It "Displays project description when specified" {
            $Output = & $Script:RbPath -P $Script:ValidProjectFile env 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join "`n") | Should -Match "Description\s*:\s*A test project for Pester testing"
        }
        
        It "Displays project file path" {
            $Output = & $Script:RbPath -P $Script:ValidProjectFile env 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join "`n") | Should -Match "Project file"
            ($Output -join "`n") | Should -Match "valid-project\.toml"
        }
        
        It "Shows correct script count" {
            $Output = & $Script:RbPath -P $Script:ValidProjectFile env 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join "`n") | Should -Match "Scripts loaded\s*:\s*4"
        }
        
        It "Lists all available scripts" {
            $Output = & $Script:RbPath -P $Script:ValidProjectFile env 2>&1
            $LASTEXITCODE | Should -Be 0
            $OutputText = $Output -join "`n"
            $OutputText | Should -Match "test"
            $OutputText | Should -Match "rspec"
            $OutputText | Should -Match "test:watch"
            $OutputText | Should -Match "guard"
            $OutputText | Should -Match "lint"
            $OutputText | Should -Match "rubocop"
        }
        
        It "Shows script descriptions when available" {
            $Output = & $Script:RbPath -P $Script:ValidProjectFile env 2>&1
            $LASTEXITCODE | Should -Be 0
            $OutputText = $Output -join "`n"
            $OutputText | Should -Match "Watch and run tests"
            $OutputText | Should -Match "Run linter"
        }
    }
    
    Context "Project File without Metadata" {
        It "Loads project file without [project] section" {
            $Output = & $Script:RbPath -P $Script:NoMetadataProjectFile env 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join "`n") | Should -Match "Project"
        }
        
        It "Does not show Name field when not specified" {
            $Output = & $Script:RbPath -P $Script:NoMetadataProjectFile env 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join "`n") | Should -Not -Match "Name\s*:"
        }
        
        It "Does not show Description field when not specified" {
            $Output = & $Script:RbPath -P $Script:NoMetadataProjectFile env 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join "`n") | Should -Not -Match "Description\s*:"
        }
        
        It "Still shows scripts from file without metadata" {
            $Output = & $Script:RbPath -P $Script:NoMetadataProjectFile env 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join "`n") | Should -Match "Scripts loaded\s*:\s*2"
        }
    }
    
    Context "Project File with Partial Metadata" {
        It "Shows only name when description is missing" {
            $Output = & $Script:RbPath -P $Script:PartialMetadataProjectFile env 2>&1
            $LASTEXITCODE | Should -Be 0
            $OutputText = $Output -join "`n"
            $OutputText | Should -Match "Name\s*:\s*Partial Metadata Project"
            $OutputText | Should -Not -Match "Description\s*:"
        }
    }
    
    Context "Empty Project File" {
        It "Handles empty file gracefully" {
            $Output = & $Script:RbPath -P $Script:EmptyFile env 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join "`n") | Should -Match "Project"
        }
        
        It "Shows zero scripts for empty file" {
            $Output = & $Script:RbPath -P $Script:EmptyFile env 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join "`n") | Should -Match "Scripts loaded\s*:\s*0"
        }
    }
    
    Context "Invalid TOML File - Error Handling" {
        It "Does not crash with invalid TOML syntax" {
            $Output = & $Script:RbPath -P $Script:InvalidTomlFile env 2>&1
            $LASTEXITCODE | Should -Be 0
        }
        
        It "Shows no project detected message for invalid TOML" {
            $Output = & $Script:RbPath -P $Script:InvalidTomlFile env 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join "`n") | Should -Match "No project config detected"
        }
        
        It "Logs warning with verbose flag for invalid TOML" {
            $Output = & $Script:RbPath -v -P $Script:InvalidTomlFile env 2>&1
            $OutputText = $Output -join "`n"
            $OutputText | Should -Match "WARN.*Failed to load"
            $OutputText | Should -Match "TOML parse error|invalid"
        }
        
        It "Still shows Ruby environment despite invalid project file" {
            $Output = & $Script:RbPath -P $Script:InvalidTomlFile env 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join "`n") | Should -Match "Ruby"
            ($Output -join "`n") | Should -Match "Environment Summary"
        }
    }
    
    Context "Non-existent File - Error Handling" {
        It "Does not crash when file does not exist" {
            $Output = & $Script:RbPath -P $Script:NonExistentFile env 2>&1
            $LASTEXITCODE | Should -Be 0
        }
        
        It "Shows no project detected message for missing file" {
            $Output = & $Script:RbPath -P $Script:NonExistentFile env 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join "`n") | Should -Match "No project config detected"
        }
        
        It "Logs warning with verbose flag for missing file" {
            $Output = & $Script:RbPath -v -P $Script:NonExistentFile env 2>&1
            $OutputText = $Output -join "`n"
            $OutputText | Should -Match "WARN.*Failed to load"
            $OutputText | Should -Match "does-not-exist\.toml"
        }
        
        It "Still shows Ruby environment despite missing project file" {
            $Output = & $Script:RbPath -P $Script:NonExistentFile env 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join "`n") | Should -Match "Ruby"
            ($Output -join "`n") | Should -Match "Environment Summary"
        }
        
        It "Shows environment ready message despite project file error" {
            $Output = & $Script:RbPath -P $Script:NonExistentFile env 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join "`n") | Should -Match "Environment ready"
        }
    }
    
    Context "Project Flag with Examples Directory" {
        It "Can load example rbproject.toml from repository" {
            $ExampleFile = Join-Path (Split-Path $Script:RbPath -Parent | Split-Path -Parent) "examples" "rbproject.toml"
            if (Test-Path $ExampleFile) {
                $Output = & $Script:RbPath -P $ExampleFile env 2>&1
                $LASTEXITCODE | Should -Be 0
                ($Output -join "`n") | Should -Match "📋 Project"
                ($Output -join "`n") | Should -Match "Ruby Butler Example Project"
            }
        }
        
        It "Shows all scripts from example file" {
            $ExampleFile = Join-Path (Split-Path $Script:RbPath -Parent | Split-Path -Parent) "examples" "rbproject.toml"
            if (Test-Path $ExampleFile) {
                $Output = & $Script:RbPath -P $ExampleFile env 2>&1
                $LASTEXITCODE | Should -Be 0
                $OutputText = $Output -join "`n"
                $OutputText | Should -Match "test.*rspec"
                $OutputText | Should -Match "lint:fix"
                $OutputText | Should -Match "Scripts loaded\s*:\s*20"
            }
        }
    }
    
    Context "Relative and Absolute Paths" {
        It "Handles relative path with .\ notation" {
            Push-Location $Script:TestDir
            try {
                $Output = & $Script:RbPath -P ".\valid-project.toml" env 2>&1
                $LASTEXITCODE | Should -Be 0
                ($Output -join "`n") | Should -Match "Project"
            } finally {
                Pop-Location
            }
        }
        
        It "Handles absolute path" {
            $Output = & $Script:RbPath -P $Script:ValidProjectFile env 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join "`n") | Should -Match "Project"
        }
    }
    
    Context "Project Flag Overrides Auto-detection" {
        It "Uses specified file even if rbproject.toml exists in current directory" {
            # Create a different rbproject.toml in temp dir
            $LocalProjectFile = Join-Path $Script:TestDir "rbproject.toml"
            @'
[project]
name = "Local Project"

[scripts]
local = "echo local"
'@ | Set-Content -Path $LocalProjectFile -Encoding UTF8
            
            Push-Location $Script:TestDir
            try {
                # Specify the valid project file (not the local one)
                $Output = & $Script:RbPath -P $Script:ValidProjectFile env 2>&1
                $LASTEXITCODE | Should -Be 0
                $OutputText = $Output -join "`n"
                # Should show the specified file, not the local one
                $OutputText | Should -Match "Test Project"
                $OutputText | Should -Not -Match "Local Project"
            } finally {
                Pop-Location
            }
        }
    }
    
    Context "Integration with Other Flags" {
        It "Works with -v verbose flag" {
            $Output = & $Script:RbPath -v -P $Script:ValidProjectFile env 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join "`n") | Should -Match "INFO"
            ($Output -join "`n") | Should -Match "Project"
        }
        
        It "Works with -vv very verbose flag" {
            $Output = & $Script:RbPath -vv -P $Script:ValidProjectFile env 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join "`n") | Should -Match "DEBUG|INFO"
            ($Output -join "`n") | Should -Match "Project"
        }
    }
}

Describe "Ruby Butler - Project Flag Error Messages" {
    Context "User-Friendly Error Messages" {
        It "Provides helpful message when file is not found" {
            $Output = & $Script:RbPath -v -P "completely-missing-file.toml" env 2>&1
            $OutputText = $Output -join "`n"
            $OutputText | Should -Match "Failed to load|cannot|not found|Systém nemůže nalézt"
        }
        
        It "Provides helpful message for parse errors" {
            $Output = & $Script:RbPath -v -P $Script:InvalidTomlFile env 2>&1
            $OutputText = $Output -join "`n"
            $OutputText | Should -Match "parse error|invalid"
        }
    }
}

Describe "Ruby Butler - gem.toml Support" {
    BeforeAll {
        # Create a gem.toml test file
        $Script:GemTomlDir = Join-Path $Script:TestDir "gem-toml-test"
        New-Item -ItemType Directory -Path $Script:GemTomlDir -Force | Out-Null
        
        $GemTomlFile = Join-Path $Script:GemTomlDir "gem.toml"
        @'
[project]
name = "Gem TOML Project"
description = "Testing gem.toml as alternative filename"

[scripts]
test = "rspec spec"
build = { command = "rake build", description = "Build the gem" }
publish = "gem push *.gem"
'@ | Set-Content -Path $GemTomlFile -Encoding UTF8
        
        # Create a directory with both rbproject.toml and gem.toml
        $Script:BothFilesDir = Join-Path $Script:TestDir "both-files-test"
        New-Item -ItemType Directory -Path $Script:BothFilesDir -Force | Out-Null
        
        $RbprojectFile = Join-Path $Script:BothFilesDir "rbproject.toml"
        @'
[project]
name = "RBProject File"

[scripts]
from-rbproject = "echo from rbproject.toml"
'@ | Set-Content -Path $RbprojectFile -Encoding UTF8
        
        $GemFile = Join-Path $Script:BothFilesDir "gem.toml"
        @'
[project]
name = "Gem File"

[scripts]
from-gem = "echo from gem.toml"
'@ | Set-Content -Path $GemFile -Encoding UTF8
    }
    
    Context "gem.toml Discovery" {
        It "Detects gem.toml in current directory" {
            Push-Location $Script:GemTomlDir
            try {
                $Output = & $Script:RbPath env 2>&1
                $LASTEXITCODE | Should -Be 0
                $OutputText = $Output -join "`n"
                $OutputText | Should -Match "gem\.toml"
                $OutputText | Should -Match "Gem TOML Project"
            } finally {
                Pop-Location
            }
        }
        
        It "Lists scripts from gem.toml with rb run" {
            Push-Location $Script:GemTomlDir
            try {
                $Output = & $Script:RbPath run 2>&1
                $LASTEXITCODE | Should -Be 0
                $OutputText = $Output -join "`n"
                $OutputText | Should -Match "test.*rspec spec"
                $OutputText | Should -Match "build.*Build the gem"
                $OutputText | Should -Match "publish"
                $OutputText | Should -Match "gem\.toml"
            } finally {
                Pop-Location
            }
        }
        
        It "Shows gem.toml scripts in rb env" {
            Push-Location $Script:GemTomlDir
            try {
                $Output = & $Script:RbPath env 2>&1
                $LASTEXITCODE | Should -Be 0
                $OutputText = $Output -join "`n"
                $OutputText | Should -Match "Project file.*gem\.toml"
                $OutputText | Should -Match "Scripts loaded.*3"
                $OutputText | Should -Match "test.*rspec spec"
                $OutputText | Should -Match "build.*rake build"
            } finally {
                Pop-Location
            }
        }
        
        It "Can specify gem.toml with -P flag" {
            $GemTomlFile = Join-Path $Script:GemTomlDir "gem.toml"
            $Output = & $Script:RbPath -P $GemTomlFile env 2>&1
            $LASTEXITCODE | Should -Be 0
            $OutputText = $Output -join "`n"
            $OutputText | Should -Match "gem\.toml"
            $OutputText | Should -Match "Gem TOML Project"
        }
    }
    
    Context "File Priority" {
        It "Prefers rbproject.toml over gem.toml when both exist" {
            Push-Location $Script:BothFilesDir
            try {
                $Output = & $Script:RbPath env 2>&1
                $LASTEXITCODE | Should -Be 0
                $OutputText = $Output -join "`n"
                # Should find rbproject.toml, not gem.toml
                # Check for the full path ending with rbproject.toml
                $OutputText | Should -Match "rbproject\.toml"
                $OutputText | Should -Match "RBProject File"
                $OutputText | Should -Not -Match "Gem File"
                $OutputText | Should -Match "from-rbproject"
                $OutputText | Should -Not -Match "from-gem"
            } finally {
                Pop-Location
            }
        }
        
        It "Uses rbproject.toml for rb run when both files exist" {
            Push-Location $Script:BothFilesDir
            try {
                $Output = & $Script:RbPath run 2>&1
                $LASTEXITCODE | Should -Be 0
                $OutputText = $Output -join "`n"
                $OutputText | Should -Match "rbproject\.toml"
                $OutputText | Should -Match "from-rbproject"
                $OutputText | Should -Not -Match "from-gem"
            } finally {
                Pop-Location
            }
        }
    }
    
    Context "Error Messages Mention Both Filenames" {
        It "Mentions both rbproject.toml and gem.toml when no config found" {
            # Create a truly isolated temp directory (not under TestDir which might have project files)
            $IsolatedDir = Join-Path $env:TEMP "rb-isolated-test-$(Get-Random)"
            New-Item -ItemType Directory -Path $IsolatedDir -Force | Out-Null
            
            Push-Location $IsolatedDir
            try {
                $Output = & $Script:RbPath run 2>&1
                $OutputText = $Output -join "`n"
                $OutputText | Should -Match "rbproject\.toml.*gem\.toml|gem\.toml.*rbproject\.toml"
            } finally {
                Pop-Location
                Remove-Item -Path $IsolatedDir -Recurse -Force -ErrorAction SilentlyContinue
            }
        }
    }
}
