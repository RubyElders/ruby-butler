# Integration Tests for Ruby Butler Sync Command
# Tests rb sync / rb s command functionality for bundler synchronization

BeforeAll {
    $Script:RbPath = $env:RB_TEST_PATH
    if (-not $Script:RbPath) {
        throw "RB_TEST_PATH environment variable not set. Run Setup.ps1 first."
    }
    
    # Create temporary directory for test files
    $Script:TestDir = Join-Path $env:TEMP "rb-sync-tests-$(Get-Random)"
    New-Item -ItemType Directory -Path $Script:TestDir -Force | Out-Null
}

AfterAll {
    # Clean up test directory
    if (Test-Path $Script:TestDir) {
        Remove-Item -Path $Script:TestDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

Describe "Ruby Butler - Sync Command" {
    Context "Sync in Bundler Project" {
        It "Successfully synchronizes bundler environment" {
            $TestSubDir = Join-Path $Script:TestDir "test-sync-$(Get-Random)"
            New-Item -ItemType Directory -Path $TestSubDir -Force | Out-Null
            
            # Create a minimal Gemfile
            @"
source 'https://rubygems.org'
gem 'rake'
"@ | Set-Content -Path (Join-Path $TestSubDir "Gemfile")
            
            Push-Location $TestSubDir
            try {
                $Output = & $Script:RbPath sync 2>&1
                $LASTEXITCODE | Should -Be 0
                ($Output -join " ") | Should -Match "Environment Successfully Synchronized|Bundle complete"
            } finally {
                Pop-Location
            }
        }
        
        It "Works with 's' alias" {
            $TestSubDir = Join-Path $Script:TestDir "test-alias-$(Get-Random)"
            New-Item -ItemType Directory -Path $TestSubDir -Force | Out-Null
            
            @"
source 'https://rubygems.org'
gem 'rake'
"@ | Set-Content -Path (Join-Path $TestSubDir "Gemfile")
            
            Push-Location $TestSubDir
            try {
                $Output = & $Script:RbPath s 2>&1
                $LASTEXITCODE | Should -Be 0
                ($Output -join " ") | Should -Match "Environment Successfully Synchronized|Bundle complete"
            } finally {
                Pop-Location
            }
        }
        
        It "Creates Gemfile.lock after sync" {
            $TestSubDir = Join-Path $Script:TestDir "test-lockfile-$(Get-Random)"
            New-Item -ItemType Directory -Path $TestSubDir -Force | Out-Null
            
            @"
source 'https://rubygems.org'
gem 'rake'
"@ | Set-Content -Path (Join-Path $TestSubDir "Gemfile")
            
            Push-Location $TestSubDir
            try {
                & $Script:RbPath sync 2>&1 | Out-Null
                Test-Path (Join-Path $TestSubDir "Gemfile.lock") | Should -Be $true
            } finally {
                Pop-Location
            }
        }
    }
    
    Context "Sync in Non-Bundler Project" {
        It "Fails gracefully when no Gemfile present" {
            $TestSubDir = Join-Path $Script:TestDir "test-no-bundler-$(Get-Random)"
            New-Item -ItemType Directory -Path $TestSubDir -Force | Out-Null
            
            Push-Location $TestSubDir
            try {
                $Output = & $Script:RbPath sync 2>&1
                $LASTEXITCODE | Should -Not -Be 0
                ($Output -join " ") | Should -Match "Bundler environment not detected"
            } finally {
                Pop-Location
            }
        }
        
        It "Fails gracefully with 's' alias when no Gemfile" {
            $TestSubDir = Join-Path $Script:TestDir "test-no-bundler-alias-$(Get-Random)"
            New-Item -ItemType Directory -Path $TestSubDir -Force | Out-Null
            
            Push-Location $TestSubDir
            try {
                $Output = & $Script:RbPath s 2>&1
                $LASTEXITCODE | Should -Not -Be 0
                ($Output -join " ") | Should -Match "Bundler environment not detected"
            } finally {
                Pop-Location
            }
        }
    }
    
    Context "Sync Updates Gemfile.lock" {
        It "Updates Gemfile.lock when gem is removed from Gemfile" {
            $TestSubDir = Join-Path $Script:TestDir "test-update-$(Get-Random)"
            New-Item -ItemType Directory -Path $TestSubDir -Force | Out-Null
            
            # Create Gemfile with two pure-Ruby gems (no native extensions)
            @"
source 'https://rubygems.org'
gem 'rake'
gem 'bundler'
"@ | Set-Content -Path (Join-Path $TestSubDir "Gemfile")
            
            Push-Location $TestSubDir
            try {
                # Initial sync
                $InitialOutput = & $Script:RbPath sync 2>&1
                
                $LockFile = Join-Path $TestSubDir "Gemfile.lock"
                
                # Verify initial sync created the lockfile
                if (-not (Test-Path $LockFile)) {
                    throw "Initial sync failed to create Gemfile.lock. Exit code: $LASTEXITCODE. Output: $($InitialOutput -join "`n")"
                }
                
                $LockContent = Get-Content $LockFile -Raw
                $LockContent | Should -Match "rake" -Because "Initial lockfile should contain rake"
                $LockContent | Should -Match "bundler" -Because "Initial lockfile should contain bundler"
                
                # Remove bundler from Gemfile
                @"
source 'https://rubygems.org'
gem 'rake'
"@ | Set-Content -Path (Join-Path $TestSubDir "Gemfile")
                
                # Sync again
                $Output = & $Script:RbPath sync 2>&1
                $LASTEXITCODE | Should -Be 0
                
                # Verify Gemfile.lock still exists and bundler is removed
                Test-Path $LockFile | Should -Be $true
                $LockContent = Get-Content $LockFile -Raw
                $LockContent | Should -Match "rake"
                $LockContent | Should -Not -Match "bundler"
            } finally {
                Pop-Location
            }
        }
    }
    
    Context "Sync with --no-bundler Flag" {
        It "Fails when --no-bundler flag is used" {
            $TestSubDir = Join-Path $Script:TestDir "test-no-bundler-flag-$(Get-Random)"
            New-Item -ItemType Directory -Path $TestSubDir -Force | Out-Null
            
            @"
source 'https://rubygems.org'
gem 'rake'
"@ | Set-Content -Path (Join-Path $TestSubDir "Gemfile")
            
            Push-Location $TestSubDir
            try {
                $Output = & $Script:RbPath --no-bundler sync 2>&1
                $LASTEXITCODE | Should -Not -Be 0
                ($Output -join " ") | Should -Match "Bundler environment not detected"
            } finally {
                Pop-Location
            }
        }
        
        It "Fails when -B flag is used" {
            $TestSubDir = Join-Path $Script:TestDir "test-b-flag-$(Get-Random)"
            New-Item -ItemType Directory -Path $TestSubDir -Force | Out-Null
            
            @"
source 'https://rubygems.org'
gem 'rake'
"@ | Set-Content -Path (Join-Path $TestSubDir "Gemfile")
            
            Push-Location $TestSubDir
            try {
                $Output = & $Script:RbPath -B sync 2>&1
                $LASTEXITCODE | Should -Not -Be 0
                ($Output -join " ") | Should -Match "Bundler environment not detected"
            } finally {
                Pop-Location
            }
        }
    }
}
