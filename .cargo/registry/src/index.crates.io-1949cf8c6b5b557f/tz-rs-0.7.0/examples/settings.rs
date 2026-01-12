fn main() -> Result<(), tz::Error> {
    #[cfg(feature = "std")]
    {
        use tz::TimeZoneSettings;

        const TIME_ZONE_SETTINGS: TimeZoneSettings<'static> =
            TimeZoneSettings::new(&["/usr/share/zoneinfo", "/share/zoneinfo", "/etc/zoneinfo"], |path| Ok(std::fs::read(path)?));

        let time_zone_local = TIME_ZONE_SETTINGS.parse_local()?;
        println!("{:?}", time_zone_local.find_current_local_time_type()?);
    }

    Ok(())
}
