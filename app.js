// 天气应用配置
const API_KEY = 'YOUR_API_KEY'; // 需要替换为你的 OpenWeatherMap API Key
const BASE_URL = 'https://api.openweathermap.org/data/2.5/weather';

// DOM 元素
const cityInput = document.getElementById('cityInput');
const searchBtn = document.getElementById('searchBtn');
const loading = document.getElementById('loading');
const errorMessage = document.getElementById('errorMessage');
const weatherContainer = document.getElementById('weatherContainer');

// 天气显示元素
const cityName = document.getElementById('cityName');
const dateTime = document.getElementById('dateTime');
const temp = document.getElementById('temp');
const weatherIcon = document.getElementById('weatherIcon');
const description = document.getElementById('description');
const feelsLike = document.getElementById('feelsLike');
const humidity = document.getElementById('humidity');
const windSpeed = document.getElementById('windSpeed');
const pressure = document.getElementById('pressure');

// 天气图标映射
const weatherIcons = {
    '01d': '☀️', '01n': '🌙',
    '02d': '⛅', '02n': '☁️',
    '03d': '☁️', '03n': '☁️',
    '04d': '☁️', '04n': '☁️',
    '09d': '🌧️', '09n': '🌧️',
    '10d': '🌦️', '10n': '🌧️',
    '11d': '⛈️', '11n': '⛈️',
    '13d': '❄️', '13n': '❄️',
    '50d': '🌫️', '50n': '🌫️'
};

// 显示加载状态
function showLoading() {
    loading.classList.remove('hidden');
    errorMessage.classList.add('hidden');
    weatherContainer.classList.add('hidden');
}

// 隐藏加载状态
function hideLoading() {
    loading.classList.add('hidden');
}

// 显示错误信息
function showError(message) {
    errorMessage.textContent = message;
    errorMessage.classList.remove('hidden');
    weatherContainer.classList.add('hidden');
}

// 显示天气数据
function showWeather() {
    errorMessage.classList.add('hidden');
    weatherContainer.classList.remove('hidden');
}

// 获取天气数据
async function getWeatherData(city) {
    try {
        showLoading();

        const response = await fetch(
            `${BASE_URL}?q=${encodeURIComponent(city)}&appid=${API_KEY}&units=metric&lang=zh_cn`
        );

        if (!response.ok) {
            if (response.status === 404) {
                throw new Error('找不到该城市，请检查城市名称');
            } else if (response.status === 401) {
                throw new Error('API Key 无效，请检查配置');
            } else {
                throw new Error(`请求失败: ${response.status}`);
            }
        }

        const data = await response.json();
        displayWeather(data);

    } catch (error) {
        showError(error.message);
    } finally {
        hideLoading();
    }
}

// 显示天气信息
function displayWeather(data) {
    // 城市和日期
    cityName.textContent = `${data.name}, ${data.sys.country}`;
    dateTime.textContent = formatDateTime(new Date());

    // 温度
    temp.textContent = Math.round(data.main.temp);

    // 天气图标和描述
    const iconCode = data.weather[0].icon;
    weatherIcon.textContent = weatherIcons[iconCode] || '🌤️';
    description.textContent = data.weather[0].description;

    // 详细信息
    feelsLike.textContent = `${Math.round(data.main.feels_like)}°C`;
    humidity.textContent = `${data.main.humidity}%`;
    windSpeed.textContent = `${data.wind.speed} m/s`;
    pressure.textContent = `${data.main.pressure} hPa`;

    showWeather();
}

// 格式化日期时间
function formatDateTime(date) {
    const options = {
        year: 'numeric',
        month: 'long',
        day: 'numeric',
        weekday: 'long',
        hour: '2-digit',
        minute: '2-digit'
    };
    return date.toLocaleDateString('zh-CN', options);
}

// 事件监听器
searchBtn.addEventListener('click', () => {
    const city = cityInput.value.trim();
    if (city) {
        getWeatherData(city);
    } else {
        showError('请输入城市名称');
    }
});

cityInput.addEventListener('keypress', (e) => {
    if (e.key === 'Enter') {
        const city = cityInput.value.trim();
        if (city) {
            getWeatherData(city);
        } else {
            showError('请输入城市名称');
        }
    }
});

// 页面加载时检查 API Key
window.addEventListener('DOMContentLoaded', () => {
    if (API_KEY === 'YOUR_API_KEY') {
        showError('请先配置 API Key！在 app.js 文件中将 YOUR_API_KEY 替换为你的 OpenWeatherMap API Key');
    }
});
