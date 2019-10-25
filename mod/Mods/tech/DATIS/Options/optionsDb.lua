local DbOption = require("Options.DbOption")

return {
  gcloudAccessKey = DbOption.new():setValue(""):editbox(),
  awsAccessKey = DbOption.new():setValue(""):editbox(),
  awsPrivateKey = DbOption.new():setValue(""):editbox(),
  awsRegion = DbOption.new():setValue("UsEast1"):combo({
    DbOption.Item(_('ApEast1')):Value('ApEast1'),
    DbOption.Item(_('ApNortheast1')):Value('ApNortheast1'),
    DbOption.Item(_('ApSoutheast1')):Value('ApSoutheast1'),
    DbOption.Item(_('ApSoutheast2')):Value('ApSoutheast2'),
    DbOption.Item(_('CaCentral1')):Value('CaCentral1'),
    DbOption.Item(_('EuCentral1')):Value('EuCentral1'),
    DbOption.Item(_('EuWest1')):Value('EuWest1'),
    DbOption.Item(_('EuWest2')):Value('EuWest2'),
    DbOption.Item(_('EuWest3')):Value('EuWest3'),
    DbOption.Item(_('EuNorth1')):Value('EuNorth1'),
    DbOption.Item(_('SaEast1')):Value('SaEast1'),
    DbOption.Item(_('UsEast1')):Value('UsEast1'),
    DbOption.Item(_('UsEast2')):Value('UsEast2'),
    DbOption.Item(_('CnNorth1')):Value('CnNorth1'),
    DbOption.Item(_('CnNorthwest1')):Value('CnNorthwest1'),
  }),
  srsPort = DbOption.new():setValue("5002"):editbox(),
  debugLoggingEnabled = DbOption.new():setValue(false):checkbox()
}
