import React from 'react';
import { Weather } from '../types';
import '../styles/CurrentWeather.css';

interface CurrentWeatherProps {
  weather: Weather;
}

const CurrentWeather: React.FC<CurrentWeatherProps> = ({ weather }) => {
  const getWeatherIcon = (description: string) => {
    const desc = description.toLowerCase();
    if (desc.includes('cloud')) return '☁️';
    if (desc.includes('rain')) return '🌧️';
    if (desc.includes('clear') || desc.includes('sunny')) return '☀️';
    if (desc.includes('snow')) return '❄️';
    if (desc.includes('wind')) return '💨';
    if (desc.includes('fog')) return '🌫️';
    return '🌤️';
  };

  return (
    <div className="current-weather">
      <div className="weather-header">
        <h2>{weather.location}</h2>
        <p className="weather-date">
          {new Date().toLocaleDateString('en-US', {
            weekday: 'long',
            year: 'numeric',
            month: 'long',
            day: 'numeric',
          })}
        </p>
      </div>

      <div className="weather-main">
        <div className="weather-icon">
          {getWeatherIcon(weather.description)}
        </div>
        <div className="weather-info">
          <div className="temperature">
            {Math.round(weather.temperature)}°C
          </div>
          <div className="description">
            {weather.description.charAt(0).toUpperCase() + weather.description.slice(1)}
          </div>
        </div>
      </div>

      <div className="weather-details">
        <div className="detail-item">
          <span className="label">Feels Like</span>
          <span className="value">{Math.round(weather.feelsLike)}°C</span>
        </div>
        <div className="detail-item">
          <span className="label">Humidity</span>
          <span className="value">{weather.humidity}%</span>
        </div>
        <div className="detail-item">
          <span className="label">Wind Speed</span>
          <span className="value">{weather.windSpeed} m/s</span>
        </div>
        <div className="detail-item">
          <span className="label">Pressure</span>
          <span className="value">{weather.pressure} hPa</span>
        </div>
      </div>
    </div>
  );
};

export default CurrentWeather;
