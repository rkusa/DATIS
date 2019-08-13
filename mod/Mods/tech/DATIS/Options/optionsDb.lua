local DbOption = require("Options.DbOption")

return {
    gcloudAccessKey = DbOption.new():setValue(""):editbox(),
    srsPort = DbOption.new():setValue("5002"):editbox(),
    debugLoggingEnabled = DbOption.new():setValue(false):checkbox()
}
