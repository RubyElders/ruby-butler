require 'mkmf'
raise 'Missing caller C++ flags' unless with_config('cxxflags') == '-std=c++17'
create_makefile('butler_cpp')
