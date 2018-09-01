-- package.path = [[]]
package.cpath = [[./target/debug/lib?.dylib;./target/debug/liblua_?_mock.dylib;]] ..
        [[./target/debug/?.dll;./target/debug/lua_?_mock.dll;]]

require 'terrain'
local datis = require 'datis'

datis.init(package.cpath)
print(type(datis.getPressure))
print(tostring(datis.getPressure()))

print("Works")