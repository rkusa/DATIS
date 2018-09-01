local Weather = require 'Weather'

getPressure = function()
    local temp, pressure = Weather.getTemperatureAndPressureAtPoint({ position = {
        x = -284887.375,
        y = 45,
        z = 683858.8125
    }})
    return pressure
end