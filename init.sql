CREATE PLUGIN weather_cache 0.2.0;
ALTER PLUGIN weather_cache 0.2.0 ADD SERVICE weather_service TO TIER default;
ALTER PLUGIN weather_cache MIGRATE TO 0.2.0;
ALTER PLUGIN weather_cache 0.2.0 ENABLE;