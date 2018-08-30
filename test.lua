-- package.path = [[]]
package.cpath = [[./target/debug/lib?.dylib;./target/debug/liblua_?_mock.dylib;]]

local dewr = require 'terrain'
local dewr = require 'dewr'

dewr.init(package.cpath)
dewr.is_visible()

local ok = false
local visible = false
local try = 0

while not ok and try < 10000 do
  ok, visbile = dewr.collect_result()
  try = try + 1
end

if ok then
  if visbile then
    print "is visible"
  else
    print "not visible"
  end
else
  print "no result within 10000 tries"
end

print("Works")