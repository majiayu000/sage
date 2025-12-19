export interface Weather {
  location: string;
  temperature: number;
  feelsLike: number;
  humidity: number;
  windSpeed: number;
  pressure: number;
  description: string;
  icon: string;
}

export interface ForecastDay {
  date: string;
  maxTemp: number;
  minTemp: number;
  description: string;
  icon: string;
}

export interface WeatherData {
  location: string;
  temperature: number;
  feelsLike: number;
  humidity: number;
  windSpeed: number;
  pressure: number;
  description: string;
  icon: string;
  forecast: ForecastDay[];
}

export interface ForecastData {
  location: string;
  forecast: ForecastDay[];
}
