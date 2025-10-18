#!/bin/bash
# ShellSpec tests for Ruby Butler project file support
# Distinguished validation of rbproject.toml and --project/-P flag

Describe "Ruby Butler Project System"
  Include spec/support/helpers.sh
  
  setup() {
    TEST_PROJECT_DIR="${SHELLSPEC_TMPBASE}/project-test-$$-${RANDOM}"
    mkdir -p "$TEST_PROJECT_DIR"
  }
  
  cleanup() {
    if [ -d "$TEST_PROJECT_DIR" ]; then
      rm -rf "$TEST_PROJECT_DIR"
    fi
  }
  
  BeforeEach 'setup'
  AfterEach 'cleanup'
  
  Describe "--project flag (-P)"
    Context "with valid project file"
      It "accepts --project flag with rbproject.toml"
        cd "$TEST_PROJECT_DIR"
        cat > custom-project.toml << 'EOF'
[project]
name = "Test Project"
description = "A test project"

[scripts]
test = "echo 'test script'"
EOF
        When run rb -R $RUBIES_DIR --project custom-project.toml env
        The status should equal 0
        The output should include "Project"
      End
      
      It "accepts -P short form flag"
        cd "$TEST_PROJECT_DIR"
        cat > custom-project.toml << 'EOF'
[project]
name = "Test Project"

[scripts]
test = "echo 'test'"
EOF
        When run rb -R $RUBIES_DIR -P custom-project.toml env
        The status should equal 0
        The output should include "Project"
      End
      
      It "displays project name from specified file"
        cd "$TEST_PROJECT_DIR"
        cat > custom-project.toml << 'EOF'
[project]
name = "Distinguished Project"
description = "A refined test project"

[scripts]
version = "ruby -v"
EOF
        When run rb -R $RUBIES_DIR -P custom-project.toml env
        The status should equal 0
        The output should include "Distinguished Project"
      End
      
      It "displays project description when specified"
        cd "$TEST_PROJECT_DIR"
        cat > custom-project.toml << 'EOF'
[project]
name = "Test"
description = "Sophisticated description text"

[scripts]
test = "echo test"
EOF
        When run rb -R $RUBIES_DIR -P custom-project.toml env
        The status should equal 0
        The output should include "Sophisticated description text"
      End
      
      It "shows --project option in help"
        When run rb --help
        The status should equal 0
        The output should include "--project"
        The output should include "rbproject.toml"
      End
    End
    
    Context "with rb run command"
      It "loads scripts from specified project file"
        cd "$TEST_PROJECT_DIR"
        cat > custom.toml << 'EOF'
[scripts]
custom-script = "echo 'custom script executed'"
EOF
        When run rb -R $RUBIES_DIR -P custom.toml run
        The status should equal 0
        The output should include "custom-script"
      End
      
      It "executes scripts from specified project file"
        Skip if "Ruby not available" is_ruby_available
        cd "$TEST_PROJECT_DIR"
        cat > custom.toml << 'EOF'
[scripts]
version = "ruby -v"
EOF
        When run rb -R $RUBIES_DIR -P custom.toml run version
        The status should equal 0
        The output should include "ruby"
      End
    End
    
    Context "with non-existent project file"
      It "handles missing project file gracefully"
        cd "$TEST_PROJECT_DIR"
        When run rb -R $RUBIES_DIR -P nonexistent.toml run
        The status should not equal 0
        The stderr should include "Selection Failed"
        The stderr should include "nonexistent.toml"
      End
    End
    
    Context "with invalid TOML"
      It "reports TOML parsing errors clearly"
        cd "$TEST_PROJECT_DIR"
        cat > invalid.toml << 'EOF'
[project
name = "Missing bracket"
EOF
        When run rb -R $RUBIES_DIR -P invalid.toml run
        The status should not equal 0
        The stderr should include "Selection Failed"
      End
    End
  End
  
  Describe "project file auto-detection"
    Context "when rbproject.toml exists in current directory"
      It "automatically discovers rbproject.toml"
        Skip if "Ruby not available" is_ruby_available
        cd "$TEST_PROJECT_DIR"
        cat > rbproject.toml << 'EOF'
[project]
name = "Auto-detected Project"

[scripts]
version = "ruby -v"
EOF
        When run rb -R $RUBIES_DIR run
        The status should equal 0
        The output should include "Auto-detected Project"
      End
      
      It "lists scripts from auto-detected file"
        cd "$TEST_PROJECT_DIR"
        cat > rbproject.toml << 'EOF'
[scripts]
test = "echo test"
build = "echo build"
EOF
        When run rb -R $RUBIES_DIR run
        The status should equal 0
        The output should include "test"
        The output should include "build"
      End
    End
    
    Context "when no rbproject.toml exists"
      It "provides helpful guidance when run command used"
        cd "$TEST_PROJECT_DIR"
        When run rb -R $RUBIES_DIR run
        The status should not equal 0
        The stderr should include "No project configuration"
        The stderr should include "rbproject.toml"
      End
    End
  End
End
