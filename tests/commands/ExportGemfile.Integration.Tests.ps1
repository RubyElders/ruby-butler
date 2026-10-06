BeforeAll {
    $Script:RbPath = $env:RB_TEST_PATH
    if (-not $Script:RbPath) {
        throw "RB_TEST_PATH environment variable not set. Run Setup.ps1 first."
    }
    $Script:Fixture = Join-Path $PSScriptRoot '../../spec/fixtures/export-gemfile/Gemfile'
}

Describe 'Ruby Butler - Gemfile export' {
    BeforeEach {
        $Script:ProjectDir = Join-Path $TestDrive 'tea-room'
        New-Item -ItemType Directory -Path $Script:ProjectDir -Force | Out-Null
        Copy-Item $Script:Fixture (Join-Path $Script:ProjectDir 'Gemfile')
        $Script:Settings = Join-Path $Script:ProjectDir 'settings.toml'
        "[project]`nname = `"Dinner service`"" | Set-Content $Script:Settings
        $Script:Errors = Join-Path $TestDrive 'stderr.txt'
    }

    It 'exports compact TOML to stdout without creating files' {
        $Output = & $Script:RbPath -r 3.4.5 -C $Script:ProjectDir export-gemfile 2> $Script:Errors
        $LASTEXITCODE | Should -Be 0
        Get-Content $Script:Errors -Raw | Should -BeNullOrEmpty
        $Text = $Output -join "`n"
        $Text | Should -Match 'name = "tea-room"'
        $Text | Should -Match 'rails = "~> 8.0"'
        $Text | Should -Match 'debug = \{ require = false \}'
        $Text | Should -Match 'url = "https://gems.example.org"'
        $Text | Should -Match 'private-api = "~> 2.0"'
        Test-Path (Join-Path $Script:ProjectDir 'rbproject.toml') | Should -BeFalse
        Test-Path (Join-Path $Script:ProjectDir 'Gemfile.lock') | Should -BeFalse
    }

    It 'exports KDL using global project settings' {
        $Original = Get-Content $Script:Settings -Raw
        $Output = & $Script:RbPath -r 3.4.5 -C $Script:ProjectDir -P settings.toml export-gemfile --format kdl 2> $Script:Errors
        $LASTEXITCODE | Should -Be 0
        Get-Content $Script:Errors -Raw | Should -BeNullOrEmpty
        $Text = $Output -join "`n"
        $Text | Should -Match 'name "Dinner service"'
        $Text | Should -Match 'gem rails "~> 8.0"'
        $Text | Should -Match 'gem debug require=#false'
        $Text | Should -Match 'source "https://gems.example.org"'
        Get-Content $Script:Settings -Raw | Should -BeExactly $Original
    }

    It 'fails when the selected Gemfile is missing' {
        $Output = & $Script:RbPath -r 3.4.5 -C $Script:ProjectDir export-gemfile missing.rb 2> $Script:Errors
        $LASTEXITCODE | Should -Not -Be 0
        $Output | Should -BeNullOrEmpty
        Get-Content $Script:Errors -Raw | Should -Match 'missing.rb'
    }
}
