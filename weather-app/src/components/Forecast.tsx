import React from 'react';
import { WeatherData } from '../types/index';
import '../styles/Forecast.css';

interface ForecastProps {
  data: WeatherData;
}

const Forecast: React.FC<ForecastProps> = ({ data }) => {
  return (
    <div className="forecast-container">
      <h3>7-Day Forecast</h3>
      <div className="forecast-grid">
        {data.forecast.map((day, index) => (
          <div key={index} className="forecast-card">
            <div className="forecast-date">
              {new Date(day.date).toLocaleDateString('en-US', {
                weekday: 'short',
                month: 'short',
                day: 'numeric',
              })}
            </div>
            <div className="forecast-icon">{day.icon}</div>
            <div className="forecast-description">{day.description}</div>
            <div className="forecast-temps">
              <span className="max-temp">{day.maxTemp}°</span>
              <span className="min-temp">{day.minTemp}°</span>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};

export default Forecast;
