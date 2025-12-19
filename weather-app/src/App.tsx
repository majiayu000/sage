import React, { useState, useEffect } from 'react';
import './App.css';
import SearchBar from './components/SearchBar';
import CurrentWeather from './components/CurrentWeather';
import Forecast from './components/Forecast';
import { fetchWeather, fetchForecast } from './services/weatherService';
import { WeatherData, ForecastData } from './types/weather';

const App: React.FC = () => {
  const [weather, setWeather] = useState<WeatherData | null>(null);
  const [forecast, setForecast] = useState<ForecastData | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [city, setCity] = useState('London');

  useEffect(() => {
    loadWeather(city);
  }, []);

  const loadWeather = async (cityName: string) => {
    setLoading(true);
    setError(null);
    try {
      const weatherData = await fetchWeather(cityName);
      const forecastData = await fetchForecast(cityName);
      setWeather(weatherData);
      setForecast(forecastData);
      setCity(cityName);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to fetch weather data');
    } finally {
      setLoading(false);
    }
  };

  const handleSearch = (cityName: string) => {
    loadWeather(cityName);
  };

  return (
    <div className="app">
      <header className="app-header">
        <h1>🌤️ Weather App</h1>
        <p>Get weather information for any location</p>
      </header>

      <SearchBar onSearch={handleSearch} />

      {error && <div className="error-message">{error}</div>}

      {loading && <div className="loading">Loading weather data...</div>}

      {weather && !loading && (
        <>
          <CurrentWeather data={weather} />
          {forecast && <Forecast data={forecast} />}
        </>
      )}
    </div>
  );
};

export default App;
