/**
 * Orderbook Depth Chart Component
 * Visualizes orderbook bid/ask levels as a depth chart
 */

'use client';

import React, { useMemo } from 'react';
import { useOrderbook } from '../../hooks/useOrderbook';
import { PriceLevel } from '../../services/orderbook';

interface OrderbookChartProps {
  marketId: string;
  outcome: number;
  className?: string;
  height?: number;
  maxLevels?: number;
}

export function OrderbookChart({
  marketId,
  outcome,
  className = '',
  height = 200,
  maxLevels = 10,
}: OrderbookChartProps) {
  const { orderbook, loading, error } = useOrderbook(marketId, outcome);

  const chartData = useMemo(() => {
    if (!orderbook) return null;

    // Take top levels from each side
    const bids = orderbook.bids.slice(0, maxLevels);
    const asks = orderbook.asks.slice(0, maxLevels);

    // Calculate cumulative sizes for depth visualization
    let cumulativeBidSize = 0;
    const bidDepth = bids.map(level => {
      cumulativeBidSize += level.size;
      return {
        price: level.price,
        size: level.size,
        cumulativeSize: cumulativeBidSize,
        side: 'bid' as const,
      };
    });

    let cumulativeAskSize = 0;
    const askDepth = asks.map(level => {
      cumulativeAskSize += level.size;
      return {
        price: level.price,
        size: level.size,
        cumulativeSize: cumulativeAskSize,
        side: 'ask' as const,
      };
    });

    // Combine and sort by price
    const allLevels = [...bidDepth, ...askDepth].sort((a, b) => a.price - b.price);

    if (allLevels.length === 0) return null;

    // Calculate price range
    const minPrice = Math.min(...allLevels.map(l => l.price));
    const maxPrice = Math.max(...allLevels.map(l => l.price));
    const maxCumulativeSize = Math.max(...allLevels.map(l => l.cumulativeSize));

    return {
      bidDepth,
      askDepth,
      allLevels,
      minPrice,
      maxPrice,
      maxCumulativeSize,
      spread: asks[0] ? asks[0].price - bids[0]?.price : 0,
      midPrice: asks[0] && bids[0] ? (asks[0].price + bids[0].price) / 2 : 50,
    };
  }, [orderbook, maxLevels]);

  if (loading) {
    return (
      <div className={`animate-pulse ${className}`} style={{ height }}>
        <div className="h-full bg-gray-600 rounded"></div>
      </div>
    );
  }

  if (error || !chartData) {
    return (
      <div className={`flex items-center justify-center text-gray-500 ${className}`} style={{ height }}>
        <div className="text-center">
          <div className="text-sm">Depth chart unavailable</div>
          <div className="text-xs mt-1">No orderbook data</div>
        </div>
      </div>
    );
  }

  const chartWidth = 400;
  const chartHeight = height - 60; // Leave space for labels

  // Generate SVG path for bid side (left side, green)
  const bidPath = chartData.bidDepth.map((level, index) => {
    const x = ((chartData.midPrice - level.price) / (chartData.maxPrice - chartData.minPrice)) * (chartWidth / 2);
    const y = chartHeight - (level.cumulativeSize / chartData.maxCumulativeSize) * chartHeight;
    return `${index === 0 ? 'M' : 'L'} ${chartWidth / 2 - x} ${y}`;
  }).join(' ');

  // Generate SVG path for ask side (right side, red)
  const askPath = chartData.askDepth.map((level, index) => {
    const x = ((level.price - chartData.midPrice) / (chartData.maxPrice - chartData.minPrice)) * (chartWidth / 2);
    const y = chartHeight - (level.cumulativeSize / chartData.maxCumulativeSize) * chartHeight;
    return `${index === 0 ? 'M' : 'L'} ${chartWidth / 2 + x} ${y}`;
  }).join(' ');

  return (
    <div className={`${className}`}>
      {/* Chart Stats */}
      <div className="grid grid-cols-3 gap-4 mb-3">
        <div className="text-center">
          <div className="text-lg font-bold text-green-400">
            {chartData.bidDepth[0]?.price.toFixed(1) || '--'}¢
          </div>
          <div className="text-xs text-gray-400">Best Bid</div>
        </div>
        <div className="text-center">
          <div className="text-lg font-bold text-white">
            {chartData.spread.toFixed(1)}¢
          </div>
          <div className="text-xs text-gray-400">Spread</div>
        </div>
        <div className="text-center">
          <div className="text-lg font-bold text-red-400">
            {chartData.askDepth[0]?.price.toFixed(1) || '--'}¢
          </div>
          <div className="text-xs text-gray-400">Best Ask</div>
        </div>
      </div>

      {/* Depth Chart */}
      <div className="relative">
        <svg
          width="100%"
          height={height}
          viewBox={`0 0 ${chartWidth} ${height}`}
          className="border border-gray-700 rounded-lg bg-gray-900/40"
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

          {/* Mid price line */}
          <line
            x1={chartWidth / 2}
            y1="0"
            x2={chartWidth / 2}
            y2={chartHeight}
            stroke="rgba(255, 255, 255, 0.3)"
            strokeWidth="1"
            strokeDasharray="2,2"
          />

          {/* Bid side depth (green, left side) */}
          <defs>
            <linearGradient id="bidGradient" x1="0%" y1="0%" x2="0%" y2="100%">
              <stop offset="0%" stopColor="#10B981" stopOpacity="0.4" />
              <stop offset="100%" stopColor="#10B981" stopOpacity="0.1" />
            </linearGradient>
            <linearGradient id="askGradient" x1="0%" y1="0%" x2="0%" y2="100%">
              <stop offset="0%" stopColor="#EF4444" stopOpacity="0.4" />
              <stop offset="100%" stopColor="#EF4444" stopOpacity="0.1" />
            </linearGradient>
          </defs>

          {/* Bid area */}
          {bidPath && (
            <>
              <path
                d={`${bidPath} L ${chartWidth / 2} ${chartHeight} L 0 ${chartHeight} Z`}
                fill="url(#bidGradient)"
              />
              <path
                d={bidPath}
                fill="none"
                stroke="#10B981"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
              />
            </>
          )}

          {/* Ask area */}
          {askPath && (
            <>
              <path
                d={`${askPath} L ${chartWidth} ${chartHeight} L ${chartWidth / 2} ${chartHeight} Z`}
                fill="url(#askGradient)"
              />
              <path
                d={askPath}
                fill="none"
                stroke="#EF4444"
                strokeWidth="2"
                strokeLinecap="round"
                strokeLinejoin="round"
              />
            </>
          )}

          {/* Price labels */}
          <text
            x="10"
            y={chartHeight + 20}
            className="fill-gray-400 text-xs"
          >
            {chartData.minPrice.toFixed(1)}¢
          </text>
          <text
            x={chartWidth / 2 - 15}
            y={chartHeight + 20}
            className="fill-white text-xs font-medium"
          >
            {chartData.midPrice.toFixed(1)}¢
          </text>
          <text
            x={chartWidth - 30}
            y={chartHeight + 20}
            className="fill-gray-400 text-xs"
          >
            {chartData.maxPrice.toFixed(1)}¢
          </text>
        </svg>

        {/* Side labels */}
        <div className="absolute top-2 left-2 text-xs text-green-400 font-medium">
          BIDS
        </div>
        <div className="absolute top-2 right-2 text-xs text-red-400 font-medium">
          ASKS
        </div>

        {/* Size label */}
        <div className="absolute top-1/2 -right-16 transform -rotate-90 text-xs text-gray-400">
          Size →
        </div>
      </div>

      {/* Legend */}
      <div className="flex justify-center items-center gap-4 mt-2 text-xs">
        <div className="flex items-center gap-1">
          <div className="w-3 h-2 bg-green-500/40 border border-green-500"></div>
          <span className="text-gray-400">Bid Depth</span>
        </div>
        <div className="flex items-center gap-1">
          <div className="w-3 h-2 bg-red-500/40 border border-red-500"></div>
          <span className="text-gray-400">Ask Depth</span>
        </div>
      </div>
    </div>
  );
}

/**
 * Mini version for smaller displays
 */
export function MiniOrderbookChart(props: Omit<OrderbookChartProps, 'height'>) {
  return <OrderbookChart {...props} height={120} maxLevels={5} />;
}