#!/bin/bash

Describe "Ruby Butler output formats"
  Include spec/support/helpers.sh

  It "uses the environment format without preparing Ruby"
    export RB_FORMAT=json
    When run rb version
    The status should equal 0
    The output should include '"schema_version":1'
    The output should include '"command":"version"'
    The stderr should equal ""
  End

  It "lets an explicit flag override the environment"
    export RB_FORMAT=json
    When run rb -F text version
    The status should equal 0
    The output should include "Ruby Butler v"
    The output should not include '"schema_version"'
    The stderr should equal ""
  End

  It "keeps completion candidates raw with JSON selected"
    export RB_FORMAT=json
    When run rb __bash_complete "rb -F " 6
    The status should equal 0
    The output should equal "text
json"
    The stderr should equal ""
  End
End
