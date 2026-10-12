require 'mkmf'
require 'middle'
raise 'transitive build dependency is missing' unless LEAF == 42
create_makefile('package_native')
