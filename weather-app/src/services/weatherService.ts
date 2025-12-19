// Weather API Service using Open-Meteo (free, no API key required)
export interface WeatherData {
  location: string;
  temperature: number;
  feelsLike: number;
  humidity: number;
  windSpeed: number;
  description: string;
  icon: string;
  forecast: ForecastDay[];
}

export interface ForecastDay {
  date: string;
  maxTemp: number;
  minTemp: number;
  description: string;
  icon: string;
}

interface GeocodingResult {
  latitude: number;
  longitude: number;
  name: string;
  country: string;
}

interface WeatherResponse {
  current: {
    temperature_2m: number;
    apparent_temperature: number;
    weather_code: number;
    relative_humidity_2m: number;
    wind_speed_10m: number;
  };
  daily: {
    time: string[];
    weather_code: number[];
    temperature_2m_max: number[];
    temperature_2m_min: number[];
  };
}

// Convert WMO weather codes to descriptions
const getWeatherDescription = (code: number): { description: string; icon: string } => {
  if (code === 0) return { description: 'Clear sky', icon: '☀️' };
  if (code === 1 || code === 2) return { description: 'Partly cloudy', icon: '⛅' };
  if (code === 3) return { description: 'Overcast', icon: '☁️' };
  if (code === 45 || code === 48) return { description: 'Foggy', icon: '🌫️' };
  if (code === 51 || code === 53 || code === 55) return { description: 'Drizzle', icon: '🌧️' };
  if (code === 61 || code === 63 || code === 65) return { description: 'Rain', icon: '🌧️' };
  if (code === 71 || code === 73 || code === 75 || code === 77 || code === 80 || code === 81 || code === 82)
    return { description: 'Snow', icon: '❄️' };
  if (code === 85 || code === 86) return { description: 'Snow showers', icon: '🌨️' };
  if (code === 80 || code === 81 || code === 82) return { description: 'Rain showers', icon: '🌧️' };
  if (code === 95 || code === 96 || code === 99) return { description: 'Thunderstorm', icon: '⛈️' };
  return { description: 'Unknown', icon: '🌡️' };
};

export const weatherService = {
  async getWeatherByLocation(location: string): Promise<WeatherData> {
    try {
      // First, geocode the location
      const geoResponse = await fetch(
        `https://geocoding-api.open-meteo.com/v1/search?name=${encodeURIComponent(location)}&count=1&language=en&format=json`
      );
      const geoData = await geoResponse.json();

      if (!geoData.results || geoData.results.length === 0) {
        throw new Error('Location not found');
      }

      const geo = geoData.results[0];
      const { latitude, longitude, name, country } = geo;

      // Get weather data
      const weatherResponse = await fetch(
        `https://api.open-meteo.com/v1/forecast?latitude=${latitude}&longitude=${longitude}&current=temperature_2m,apparent_temperature,weather_code,relative_humidity_2m,wind_speed_10m&daily=weather_code,temperature_2m_max,temperature_2m_min&temperature_unit=celsius&wind_speed_unit=kmh&timezone=auto`
      );
      const weatherData: WeatherResponse = await weatherResponse.json();

      const current = weatherData.current;
      const { description, icon } = getWeatherDescription(current.weather_code);

      // Build forecast
      const forecast: ForecastDay[] = weatherData.daily.time.slice(0, 7).map((date, idx) => {
        const { description: forecastDesc, icon: forecastIcon } = getWeatherDescription(
          weatherData.daily.weather_code[idx]
        );
        return {
          date,
          maxTemp: Math.round(weatherData.daily.temperature_2m_max[idx]),
          minTemp: Math.round(weatherData.daily.temperature_2m_min[idx]),
          description: forecastDesc,
          icon: forecastIcon,
        };
      });

      return {
        location: `${name}, ${country}`,
        temperature: Math.round(current.temperature_2m),
        feelsLike: Math.round(current.apparent_temperature),
        humidity: current.relative_humidity_2m,
        windSpeed: Math.round(current.wind_speed_10m),
        description,
        icon,
        forecast,
      };
    } catch (error) {
      throw new Error(`Failed to fetch weather: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  },
};
