'use client';

import React, { useState } from 'react';
import { TrendingUp, Calendar, BarChart3 } from 'lucide-react';

interface MarketChartProps {
  marketId: string;
  probability: number;
}

// Mock data for demonstration
const generateMockData = (probability: number) => {
  const now = Date.now();
  const data = [];
  let currentPrice = probability * 100;
  
  // Generate 30 data points over the last 30 days
  for (let i = 30; i >= 0; i--) {
    const timestamp = now - (i * 24 * 60 * 60 * 1000);
    // Add some realistic price movement
    const volatility = (Math.random() - 0.5) * 8; // ±4% volatility
    currentPrice = Math.max(5, Math.min(95, currentPrice + volatility));
    
    data.push({
      timestamp,
      date: new Date(timestamp).toLocaleDateString(),
      price: currentPrice,
      volume: Math.floor(Math.random() * 10000) + 1000
    });
  }
  
  return data;
};

export function MarketChart({ marketId, probability }: MarketChartProps) {
  const [timeframe, setTimeframe] = useState<'24h' | '7d' | '30d' | 'all'>('7d');
  const [chartType, setChartType] = useState<'price' | 'volume'>('price');
  
  const data = generateMockData(probability);
  const currentPrice = data[data.length - 1].price;
  const previousPrice = data[data.length - 2].price;
  const priceChange = currentPrice - previousPrice;
  const priceChangePercent = ((priceChange / previousPrice) * 100);

  // Calculate SVG path for the price line
  const chartWidth = 300;
  const chartHeight = 120;
  const minPrice = Math.min(...data.map(d => d.price));
  const maxPrice = Math.max(...data.map(d => d.price));
  const priceRange = maxPrice - minPrice;
  
  const pathData = data.map((point, index) => {
    const x = (index / (data.length - 1)) * chartWidth;
    const y = chartHeight - ((point.price - minPrice) / priceRange) * chartHeight;
    return `${index === 0 ? 'M' : 'L'} ${x} ${y}`;
  }).join(' ');

  return (
    <div className="futurecast-card p-4">
      <div className="flex items-center justify-between mb-4">
        <h3 className="text-lg font-bold text-white">Price Chart</h3>
        <div className="flex items-center gap-2">
          <div className="flex bg-gray-800/50 rounded-lg p-1">
            {['24h', '7d', '30d', 'all'].map((period) => (
              <button
                key={period}
                onClick={() => setTimeframe(period as any)}
                className={`px-2 py-1 rounded text-xs font-medium transition-colors ${
                  timeframe === period
                    ? 'bg-red-500 text-white'
                    : 'text-gray-400 hover:text-white'
                }`}
              >
                {period}
              </button>
            ))}
          </div>
        </div>
      </div>

      {/* Price Stats */}
      <div className="grid grid-cols-3 gap-4 mb-4">
        <div className="text-center">
          <div className="text-2xl font-bold text-white">
            {currentPrice.toFixed(1)}¢
          </div>
          <div className="text-xs text-gray-400">Current Price</div>
        </div>
        <div className="text-center">
          <div className={`text-lg font-semibold flex items-center justify-center gap-1 ${
            priceChange >= 0 ? 'text-green-400' : 'text-red-400'
          }`}>
            <TrendingUp className={`w-4 h-4 ${priceChange < 0 ? 'rotate-180' : ''}`} />
            {priceChangePercent >= 0 ? '+' : ''}{priceChangePercent.toFixed(2)}%
          </div>
          <div className="text-xs text-gray-400">24h Change</div>
        </div>
        <div className="text-center">
          <div className="text-lg font-semibold text-blue-400">
            {data[data.length - 1].volume.toLocaleString()}
          </div>
          <div className="text-xs text-gray-400">24h Volume</div>
        </div>
      </div>

      {/* Chart */}
      <div className="relative">
        <svg
          width="100%"
          height="120"
          viewBox={`0 0 ${chartWidth} ${chartHeight}`}
          className="border border-gray-700 rounded-lg bg-gray-800/20"
        >
          {/* Grid lines */}
          {[0, 0.25, 0.5, 0.75, 1].map((ratio) => (
            <line
              key={ratio}
              x1="0"
              y1={chartHeight * ratio}
              x2={chartWidth}
              y2={chartHeight * ratio}
              stroke="rgba(255, 255, 255, 0.1)"
              strokeWidth="0.5"
            />
          ))}
          
          {/* Price line */}
          <path
            d={pathData}
            fill="none"
            stroke={priceChange >= 0 ? "#10B981" : "#EF4444"}
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          />
          
          {/* Gradient fill */}
          <defs>
            <linearGradient id={`gradient-${marketId}`} x1="0%" y1="0%" x2="0%" y2="100%">
              <stop offset="0%" stopColor={priceChange >= 0 ? "#10B981" : "#EF4444"} stopOpacity="0.3" />
              <stop offset="100%" stopColor={priceChange >= 0 ? "#10B981" : "#EF4444"} stopOpacity="0.05" />
            </linearGradient>
          </defs>
          <path
            d={`${pathData} L ${chartWidth} ${chartHeight} L 0 ${chartHeight} Z`}
            fill={`url(#gradient-${marketId})`}
          />
          
          {/* Current price indicator */}
          <circle
            cx={chartWidth}
            cy={chartHeight - ((currentPrice - minPrice) / priceRange) * chartHeight}
            r="3"
            fill={priceChange >= 0 ? "#10B981" : "#EF4444"}
          />
        </svg>
        
        {/* Price labels */}
        <div className="absolute -right-12 top-0 text-xs text-gray-400">
          {maxPrice.toFixed(0)}¢
        </div>
        <div className="absolute -right-12 bottom-0 text-xs text-gray-400">
          {minPrice.toFixed(0)}¢
        </div>
      </div>

      {/* Chart Type Toggle */}
      <div className="flex justify-center mt-3">
        <div className="flex bg-gray-800/50 rounded-lg p-1">
          <button
            onClick={() => setChartType('price')}
            className={`px-3 py-1 rounded text-xs font-medium flex items-center gap-1 transition-colors ${
              chartType === 'price'
                ? 'bg-red-500 text-white'
                : 'text-gray-400 hover:text-white'
            }`}
          >
            <TrendingUp className="w-3 h-3" />
            Price
          </button>
          <button
            onClick={() => setChartType('volume')}
            className={`px-3 py-1 rounded text-xs font-medium flex items-center gap-1 transition-colors ${
              chartType === 'volume'
                ? 'bg-red-500 text-white'
                : 'text-gray-400 hover:text-white'
            }`}
          >
            <BarChart3 className="w-3 h-3" />
            Volume
          </button>
        </div>
      </div>
    </div>
  );
}