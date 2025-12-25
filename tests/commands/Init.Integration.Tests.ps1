# Integration Tests for Ruby Butler Init Command
# Tests rb init command functionality for creating rbproject.toml files

BeforeAll {
    $Script:RbPath = $env:RB_TEST_PATH
    if (-not $Script:RbPath) {
        throw "RB_TEST_PATH environment variable not set. Run Setup.ps1 first."
    }
    
    # Create temporary directory for test files
    $Script:TestDir = Join-Path $env:TEMP "rb-init-tests-$(Get-Random)"
    New-Item -ItemType Directory -Path $Script:TestDir -Force | Out-Null
}

AfterAll {
    # Clean up test directory
    if (Test-Path $Script:TestDir) {
        Remove-Item -Path $Script:TestDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

Describe "Ruby Butler - Init Command" {
    Context "Creating New rbproject.toml" {
        It "Creates rbproject.toml in current directory" {
            $TestSubDir = Join-Path $Script:TestDir "test-init-$(Get-Random)"
            New-Item -ItemType Directory -Path $TestSubDir -Force | Out-Null
            
            Push-Location $TestSubDir
            try {
                $Output = & $Script:RbPath init 2>&1
                $LASTEXITCODE | Should -Be 0
                ($Output -join " ") | Should -Match "Splendid"
                Test-Path (Join-Path $TestSubDir "rbproject.toml") | Should -Be $true
            } finally {
                Pop-Location
            }
        }
        
        It "Displays success message with ceremony" {
            $TestSubDir = Join-Path $Script:TestDir "test-success-$(Get-Random)"
            New-Item -ItemType Directory -Path $TestSubDir -Force | Out-Null
            
            Push-Location $TestSubDir
            try {
                $Output = & $Script:RbPath init 2>&1
                $LASTEXITCODE | Should -Be 0
                ($Output -join " ") | Should -Match "Splendid"
                ($Output -join " ") | Should -Match "rbproject.toml has been created"
            } finally {
                Pop-Location
            }
        }
        
        It "Creates valid TOML file" {
            $TestSubDir = Join-Path $Script:TestDir "test-valid-$(Get-Random)"
            New-Item -ItemType Directory -Path $TestSubDir -Force | Out-Null
            
            Push-Location $TestSubDir
            try {
                & $Script:RbPath init 2>&1 | Out-Null
                $Content = Get-Content (Join-Path $TestSubDir "rbproject.toml") -Raw
                $Content | Should -Match "\[project\]"
                $Content | Should -Match "\[scripts\]"
            } finally {
                Pop-Location
            }
        }
        
        It "Includes project metadata section" {
            $TestSubDir = Join-Path $Script:TestDir "test-metadata-$(Get-Random)"
            New-Item -ItemType Directory -Path $TestSubDir -Force | Out-Null
            
            Push-Location $TestSubDir
            try {
                & $Script:RbPath init 2>&1 | Out-Null
                $Content = Get-Content (Join-Path $TestSubDir "rbproject.toml") -Raw
                $Content | Should -Match 'name = "Butler project template"'
                $Content | Should -Match 'description'
            } finally {
                Pop-Location
            }
        }
        
        It "Includes sample ruby-version script" {
            $TestSubDir = Join-Path $Script:TestDir "test-script-$(Get-Random)"
            New-Item -ItemType Directory -Path $TestSubDir -Force | Out-Null
            
            Push-Location $TestSubDir
            try {
                & $Script:RbPath init 2>&1 | Out-Null
                $Content = Get-Content (Join-Path $TestSubDir "rbproject.toml") -Raw
                $Content | Should -Match 'ruby-version = "ruby -v"'
            } finally {
                Pop-Location
            }
        }
        
        It "Provides helpful next steps" {
            $TestSubDir = Join-Path $Script:TestDir "test-nextsteps-$(Get-Random)"
            New-Item -ItemType Directory -Path $TestSubDir -Force | Out-Null
            
            Push-Location $TestSubDir
            try {
                $Output = & $Script:RbPath init 2>&1
                $LASTEXITCODE | Should -Be 0
                ($Output -join " ") | Should -Match "You may now"
                ($Output -join " ") | Should -Match "rb run"
            } finally {
                Pop-Location
            }
        }
        
        It "References example documentation" {
            $TestSubDir = Join-Path $Script:TestDir "test-examples-$(Get-Random)"
            New-Item -ItemType Directory -Path $TestSubDir -Force | Out-Null
            
            Push-Location $TestSubDir
            try {
                $Output = & $Script:RbPath init 2>&1
                $LASTEXITCODE | Should -Be 0
                ($Output -join " ") | Should -Match "examples/rbproject.toml"
            } finally {
                Pop-Location
            }
        }
    }
    
    Context "When rbproject.toml Already Exists" {
        It "Gracefully refuses to overwrite existing file" {
            $TestSubDir = Join-Path $Script:TestDir "test-exists-$(Get-Random)"
            New-Item -ItemType Directory -Path $TestSubDir -Force | Out-Null
            
            $ProjectFile = Join-Path $TestSubDir "rbproject.toml"
            "existing content" | Set-Content -Path $ProjectFile
            
            Push-Location $TestSubDir
            try {
                $Output = & $Script:RbPath init 2>&1
                $LASTEXITCODE | Should -Not -Be 0
                ($Output -join " ") | Should -Match "already graces"
                ($Output -join " ") | Should -Match "this directory"
            } finally {
                Pop-Location
            }
        }
        
        It "Provides proper guidance for resolution" {
            $TestSubDir = Join-Path $Script:TestDir "test-guidance-$(Get-Random)"
            New-Item -ItemType Directory -Path $TestSubDir -Force | Out-Null
            
            $ProjectFile = Join-Path $TestSubDir "rbproject.toml"
            "existing content" | Set-Content -Path $ProjectFile
            
            Push-Location $TestSubDir
            try {
                $Output = & $Script:RbPath init 2>&1
                $LASTEXITCODE | Should -Not -Be 0
                ($Output -join " ") | Should -Match "delete the existing one first"
            } finally {
                Pop-Location
            }
        }
        
        It "Preserves existing file content" {
            $TestSubDir = Join-Path $Script:TestDir "test-preserve-$(Get-Random)"
            New-Item -ItemType Directory -Path $TestSubDir -Force | Out-Null
            
            $ProjectFile = Join-Path $TestSubDir "rbproject.toml"
            "my precious content" | Set-Content -Path $ProjectFile
            
            Push-Location $TestSubDir
            try {
                & $Script:RbPath init 2>&1 | Out-Null
                $Content = Get-Content $ProjectFile -Raw
                $Content | Should -BeExactly "my precious content`r`n"
            } finally {
                Pop-Location
            }
        }
    }
    
    Context "Working with Generated rbproject.toml" {
        It "Can list scripts from generated file" {
            $TestSubDir = Join-Path $Script:TestDir "test-list-$(Get-Random)"
            New-Item -ItemType Directory -Path $TestSubDir -Force | Out-Null
            
            Push-Location $TestSubDir
            try {
                & $Script:RbPath init 2>&1 | Out-Null
                $Output = & $Script:RbPath run 2>&1
                $LASTEXITCODE | Should -Be 0
                ($Output -join " ") | Should -Match "ruby-version"
            } finally {
                Pop-Location
            }
        }
    }
}
