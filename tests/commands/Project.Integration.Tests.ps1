# Integration Tests for Ruby Butler Project Command
# Tests rb info project functionality and rbproject.toml handling

BeforeAll {
    $Script:RbPath = $env:RB_TEST_PATH
    if (-not $Script:RbPath) {
        throw "RB_TEST_PATH environment variable not set. Run Setup.ps1 first."
    }
    
    $Script:TestDir = Join-Path $env:TEMP "rb-project-tests-$([System.Random]::new().Next())"
    New-Item -ItemType Directory -Path $Script:TestDir -Force | Out-Null
}

AfterAll {
    if (Test-Path $Script:TestDir) {
        Remove-Item -Path $Script:TestDir -Recurse -Force
    }
}

Describe "Ruby Butler - Project Command Integration" {
    Context "--project flag (-P)" {
        It "Accepts --project flag with rbproject.toml" {
            $TestSubDir = Join-Path $Script:TestDir "test-project-flag-$(Get-Random)"
            New-Item -ItemType Directory -Path $TestSubDir -Force | Out-Null
            
            $ProjectFile = Join-Path $TestSubDir "custom-project.toml"
            @"
[project]
name = "Test Project"
description = "A test project"

[scripts]
test = "echo 'test script'"
"@ | Set-Content -Path $ProjectFile
            
            Push-Location $TestSubDir
            try {
                $Output = & $Script:RbPath --project custom-project.toml info env 2>&1
                $LASTEXITCODE | Should -Be 0
                ($Output -join " ") | Should -Match "Project|Test Project"
            } finally {
                Pop-Location
            }
        }
        
        It "Accepts -P short form flag" {
            $TestSubDir = Join-Path $Script:TestDir "test-p-flag-$(Get-Random)"
            New-Item -ItemType Directory -Path $TestSubDir -Force | Out-Null
            
            $ProjectFile = Join-Path $TestSubDir "custom-project.toml"
            @"
[project]
name = "Test Project"

[scripts]
test = "echo 'test'"
"@ | Set-Content -Path $ProjectFile
            
            Push-Location $TestSubDir
            try {
                $Output = & $Script:RbPath -P custom-project.toml info env 2>&1
                $LASTEXITCODE | Should -Be 0
                ($Output -join " ") | Should -Match "Project|Test Project"
            } finally {
                Pop-Location
            }
        }
        
        It "Displays project name from specified file with info project" {
            $TestSubDir = Join-Path $Script:TestDir "test-project-name-$(Get-Random)"
            New-Item -ItemType Directory -Path $TestSubDir -Force | Out-Null
            
            $ProjectFile = Join-Path $TestSubDir "custom-project.toml"
            @"
[project]
name = "Distinguished Project"
description = "A refined test project"

[scripts]
version = "ruby -v"
"@ | Set-Content -Path $ProjectFile
            
            Push-Location $TestSubDir
            try {
                $Output = & $Script:RbPath -P custom-project.toml info project 2>&1
                $LASTEXITCODE | Should -Be 0
                ($Output -join " ") | Should -Match "Distinguished Project"
            } finally {
                Pop-Location
            }
        }
        
        It "Displays project description when specified" {
            $TestSubDir = Join-Path $Script:TestDir "test-project-desc-$(Get-Random)"
            New-Item -ItemType Directory -Path $TestSubDir -Force | Out-Null
            
            $ProjectFile = Join-Path $TestSubDir "custom-project.toml"
            @"
[project]
name = "Test"
description = "Sophisticated description text"

[scripts]
test = "echo test"
"@ | Set-Content -Path $ProjectFile
            
            Push-Location $TestSubDir
            try {
                $Output = & $Script:RbPath -P custom-project.toml info project 2>&1
                $LASTEXITCODE | Should -Be 0
                ($Output -join " ") | Should -Match "Sophisticated description text"
            } finally {
                Pop-Location
            }
        }
        
        It "Shows --project option in help" {
            $Output = & $Script:RbPath help 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "--project"
            ($Output -join " ") | Should -Match "rbproject.toml"
        }
    }
    
    Context "with rb run command" {
        It "Loads scripts from specified project file" {
            $TestSubDir = Join-Path $Script:TestDir "test-run-scripts-$(Get-Random)"
            New-Item -ItemType Directory -Path $TestSubDir -Force | Out-Null
            
            $ProjectFile = Join-Path $TestSubDir "custom.toml"
            @"
[scripts]
custom-script = "echo 'custom script executed'"
"@ | Set-Content -Path $ProjectFile
            
            Push-Location $TestSubDir
            try {
                $Output = & $Script:RbPath -P custom.toml run 2>&1
                $LASTEXITCODE | Should -Be 0
                ($Output -join " ") | Should -Match "custom-script"
            } finally {
                Pop-Location
            }
        }
    }
    
    Context "with non-existent project file" {
        It "Handles missing project file gracefully" {
            $TestSubDir = Join-Path $Script:TestDir "test-missing-$(Get-Random)"
            New-Item -ItemType Directory -Path $TestSubDir -Force | Out-Null
            
            Push-Location $TestSubDir
            try {
                $Output = & $Script:RbPath -P nonexistent.toml run 2>&1
                $LASTEXITCODE | Should -Not -Be 0
                ($Output -join " ") | Should -Match "could not be loaded"
                ($Output -join " ") | Should -Match "nonexistent.toml"
            } finally {
                Pop-Location
            }
        }
    }
    
    Context "project file auto-detection" {
        It "Automatically discovers rbproject.toml" {
            $TestSubDir = Join-Path $Script:TestDir "test-autodetect-$(Get-Random)"
            New-Item -ItemType Directory -Path $TestSubDir -Force | Out-Null
            
            $ProjectFile = Join-Path $TestSubDir "rbproject.toml"
            @"
[project]
name = "Auto-detected Project"

[scripts]
version = "ruby -v"
"@ | Set-Content -Path $ProjectFile
            
            Push-Location $TestSubDir
            try {
                $Output = & $Script:RbPath run 2>&1
                $LASTEXITCODE | Should -Be 0
                ($Output -join " ") | Should -Match "Auto-detected Project"
            } finally {
                Pop-Location
            }
        }
        
        It "Lists scripts from auto-detected file" {
            $TestSubDir = Join-Path $Script:TestDir "test-list-scripts-$(Get-Random)"
            New-Item -ItemType Directory -Path $TestSubDir -Force | Out-Null
            
            $ProjectFile = Join-Path $TestSubDir "rbproject.toml"
            @"
[scripts]
test = "echo test"
build = "echo build"
"@ | Set-Content -Path $ProjectFile
            
            Push-Location $TestSubDir
            try {
                $Output = & $Script:RbPath run 2>&1
                $LASTEXITCODE | Should -Be 0
                ($Output -join " ") | Should -Match "test"
                ($Output -join " ") | Should -Match "build"
            } finally {
                Pop-Location
            }
        }
    }
    
    Context "when no rbproject.toml exists" {
        It "Provides helpful guidance when run command used" {
            $TestSubDir = Join-Path $Script:TestDir "test-no-project-$(Get-Random)"
            New-Item -ItemType Directory -Path $TestSubDir -Force | Out-Null
            
            Push-Location $TestSubDir
            try {
                $Output = & $Script:RbPath run 2>&1
                $LASTEXITCODE | Should -Not -Be 0
                ($Output -join " ") | Should -Match "No project configuration"
                ($Output -join " ") | Should -Match "rbproject.toml"
            } finally {
                Pop-Location
            }
        }
    }
}
