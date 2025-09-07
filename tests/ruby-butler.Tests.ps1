#!/usr/bin/env pwsh

# Pester Tests for Ruby Butler
# Basic functionality tests for Windows PowerShell
# Usage: Invoke-Pester ruby-butler.Tests.ps1

BeforeAll {
    # Build the project
    Write-Host "Building Ruby Butler..." -ForegroundColor Yellow
    $BuildResult = & cargo build --release 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "Build failed: $BuildResult"
    }
    
    # Set up the executable path
    $Script:RbPath = Join-Path $PSScriptRoot "..\target\release\rb.exe"
    if (-not (Test-Path $Script:RbPath)) {
        throw "rb.exe not found at: $Script:RbPath"
    }
    
    Write-Host "✅ Build successful! Testing: $Script:RbPath" -ForegroundColor Green
}

Describe "Ruby Butler - Basic Functionality" {
    Context "Help System" {
        It "Shows help with --help flag" {
            $Output = & $Script:RbPath --help
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "Ruby Butler"
        }
        
        It "Shows help with -h flag" {
            $Output = & $Script:RbPath -h
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "Ruby Butler"
        }
        
        It "Lists main commands in help" {
            $Output = & $Script:RbPath --help
            $OutputText = $Output -join " "
            $OutputText | Should -Match "runtime"
            $OutputText | Should -Match "environment"
            $OutputText | Should -Match "exec"
            $OutputText | Should -Match "sync"
        }
    }
    
    Context "Version Information" {
        It "Shows version with --version flag" {
            $Output = & $Script:RbPath --version
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "rb \d+\.\d+\.\d+"
        }
        
        It "Shows version with -V flag" {
            $Output = & $Script:RbPath -V
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "rb \d+\.\d+\.\d+"
        }
    }
    
    Context "Command Recognition" {
        It "Recognizes runtime command" {
            $Output = & $Script:RbPath runtime --help 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "Survey.*Ruby.*estate"
        }
        
        It "Recognizes rt alias for runtime" {
            $Output = & $Script:RbPath rt --help 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "Survey.*Ruby.*estate"
        }
        
        It "Recognizes environment command" {
            $Output = & $Script:RbPath environment --help 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "Present.*Ruby.*environment"
        }
        
        It "Recognizes env alias for environment" {
            $Output = & $Script:RbPath env --help 2>&1
            $LASTEXITCODE | Should -Be 0
            ($Output -join " ") | Should -Match "Present.*Ruby.*environment"
        }
    }
    
    Context "Gentleman's Approach" {
        It "Uses sophisticated language" {
            $Output = & $Script:RbPath --help
            $OutputText = $Output -join " "
            $OutputText | Should -Match "sophisticated|distinguished|meticulously|refined"
        }
        
        It "Presents as environment manager, not version switcher" {
            $Output = & $Script:RbPath --help
            $OutputText = $Output -join " "
            $OutputText | Should -Match "environment manager"
            # The text says "Not merely a version switcher" which is good - it's distinguishing itself
            $OutputText | Should -Match "Not merely a version switcher"
        }
        
        It "Includes RubyElders branding" {
            $Output = & $Script:RbPath --help
            $OutputText = $Output -join " "
            $OutputText | Should -Match "RubyElders"
        }
    }
    
    Context "Error Handling" {
        It "Handles invalid commands gracefully" {
            $Output = & $Script:RbPath invalid-command 2>&1
            $LASTEXITCODE | Should -Not -Be 0
            # Should give some kind of error or help message
            $Output | Should -Not -BeNullOrEmpty
        }
        
        It "Handles invalid options gracefully" {
            $Output = & $Script:RbPath --invalid-option 2>&1
            $LASTEXITCODE | Should -Not -Be 0
            # Should give some kind of error or help message
            $Output | Should -Not -BeNullOrEmpty
        }
    }
}
