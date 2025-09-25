'use client';

import React, { useState, useEffect } from 'react';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Market } from '@/lib/near';
import { formatTime, formatRelativeTime, getMarketStatusColor, getMarketStatusText, formatCurrency, calculateProbability } from '@/lib/utils';
import { Calendar, Clock, TrendingUp, Users, Volume2, ArrowUp, ArrowDown } from 'lucide-react';
import Link from 'next/link';
import { LivePriceDisplay, SimplePrice, PriceChange } from '@/components/orderbook/LivePriceDisplay';
import { MiniOrderbookChart } from '@/components/orderbook/OrderbookChart';
import { useMarketPrice } from '@/hooks/useOrderbook';

interface MarketCardProps {
  market: Market;
  showTradingButtons?: boolean;
  compact?: boolean;
}

// Component for dynamic community prediction based on actual market prices
function CommunityPredictionBar({ marketId }: { marketId: string }) {
  const { price: yesPrice, loading } = useMarketPrice(marketId, 1);
  const [showFallback, setShowFallback] = useState(false);

  // Add delay before showing fallback values
  useEffect(() => {
    const timer = setTimeout(() => {
      setShowFallback(true);
    }, 1500);

    return () => clearTimeout(timer);
  }, [marketId]);

  // Reset fallback when market changes
  useEffect(() => {
    setShowFallback(false);
  }, [marketId]);

  // Calculate percentage from YES price (price is already in cents, so it's the percentage)
  const yesPercentage = yesPrice?.mid || (showFallback ? 50 : null);
  const noPercentage = yesPercentage ? 100 - yesPercentage : null;

  return (
    <div className="bg-gray-800/30 rounded-xl p-3 mb-4">
      <div className="flex items-center justify-between mb-2">
        <span className="text-gray-300 text-sm">Community Prediction</span>
        <TrendingUp className="w-4 h-4 text-green-400" />
      </div>
      <div className="w-full bg-gray-700 rounded-full h-2">
        {yesPercentage !== null ? (
          <div 
            className="bg-gradient-to-r from-green-500 to-blue-500 h-2 rounded-full transition-all duration-300"
            style={{ width: `${yesPercentage}%` }}
          />
        ) : (
          <div className="bg-gray-500/30 h-2 rounded-full animate-pulse" />
        )}
      </div>
      <div className="flex justify-between text-xs text-gray-400 mt-2">
        <span>
          {yesPercentage !== null ? `${yesPercentage.toFixed(0)}% YES` : 
           showFallback ? '50% YES' : 
           <span className="animate-pulse">-- YES</span>}
        </span>
        <span>
          {noPercentage !== null ? `${noPercentage.toFixed(0)}% NO` : 
           showFallback ? '50% NO' : 
           <span className="animate-pulse">-- NO</span>}
        </span>
      </div>
    </div>
  );
}

export function MarketCard({ market, showTradingButtons = false, compact = false }: MarketCardProps) {
  const [loading, setLoading] = useState(false);
  
  const status = getMarketStatusText(market.is_active, market.end_time);
  
  const volume = market.total_volume || '0';

  const getCategoryClass = (category: string) => {
    const categoryLower = category.toLowerCase();
    switch (categoryLower) {
      case 'crypto': return 'category-crypto';
      case 'politics': return 'category-politics';
      case 'sports': return 'category-sports';
      case 'technology': case 'tech': return 'category-tech';
      default: return 'category-default';
    }
  };

  const getStatusClass = (status: string) => {
    switch (status.toLowerCase()) {
      case 'active': return 'status-live';
      case 'closed': return 'status-closed';
      case 'resolved': return 'status-resolved';
      default: return 'status-live';
    }
  };

  const handleTrade = async (outcome: 'YES' | 'NO') => {
    setLoading(true);
    try {
      window.location.href = `/market/${market.market_id}?trade=${outcome}`;
    } catch (error) {
      console.error('Error initiating trade:', error);
    } finally {
      setLoading(false);
    }
  };

  // Live Price Component with fallback
  const LivePriceComponent = ({ className = "" }: { className?: string }) => (
    <SimplePrice
      marketId={market.market_id}
      outcome={1}
      className={className}
      showCurrency={true}
    />
  );

  // Fallback price display
  const FallbackPrice = ({ className = "" }: { className?: string }) => (
    <span className={className}>50¢</span>
  );

  if (compact) {
    return (
      <div className="polymarket-card p-4 cursor-pointer fade-in">
        <div className="flex items-center justify-between mb-3">
          <div className={getCategoryClass(market.category)}>
            {market.category.toUpperCase()}
          </div>
          <div className={getStatusClass(status)}>
            {status.toUpperCase()}
          </div>
        </div>
        
        <Link href={`/market/${market.market_id}`}>
          <h3 className="text-white font-semibold mb-3 hover:text-indigo-400 transition-colors line-clamp-2">
            {market.title}
          </h3>
        </Link>
        
        <div className="flex items-center justify-between">
          <div>
            <div className="price-display text-green-400 text-xl">
              <LivePriceComponent className="text-green-400 text-xl" />
            </div>
            <div className="text-xs text-gray-400 flex items-center gap-1">
              Yes
              <PriceChange marketId={market.market_id} outcome={1} className="text-xs" />
            </div>
          </div>
          <div className="text-right">
            <div className="text-sm text-white font-medium">
              {formatCurrency(parseFloat(volume) / 1e6, 'USD', 0)}
            </div>
            <div className="text-xs text-gray-400">Volume</div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="polymarket-card p-6 cursor-pointer fade-in">
      {/* Header */}
      <div className="flex items-center justify-between mb-4">
        <div className={getCategoryClass(market.category)}>
          {market.category.toUpperCase()}
        </div>
        <div className={getStatusClass(status)}>
          {status.toUpperCase()}
        </div>
      </div>

      {/* Title */}
      <Link href={`/market/${market.market_id}`}>
        <h3 className="text-white font-semibold text-lg mb-4 hover:text-indigo-400 transition-colors line-clamp-2">
          {market.title}
        </h3>
      </Link>

      {/* Description */}
      <p className="text-gray-400 text-sm mb-6 line-clamp-2">
        {market.description}
      </p>

      {/* Price Section */}
      <div className="flex items-center justify-between mb-6">
        <div>
          <div className="flex items-center gap-2 mb-1">
            <div className="price-display text-green-400">
              <LivePriceComponent className="text-green-400" />
            </div>
            <PriceChange marketId={market.market_id} outcome={1} />
          </div>
          <div className="text-xs text-gray-400">Yes • Live</div>
        </div>

        <div className="text-right">
          <div className="text-lg font-semibold text-white">
            {formatCurrency(parseFloat(volume) / 1e6, 'USD', 0)}
          </div>
          <div className="text-xs text-gray-400">Volume</div>
        </div>
      </div>

      {/* Community Prediction */}
      <CommunityPredictionBar marketId={market.market_id} />

      {/* Mini Depth Chart */}
      <div className="mb-6">
        <MiniOrderbookChart
          marketId={market.market_id}
          outcome={1}
          maxLevels={5}
          className="opacity-80"
        />
      </div>

      {/* Trading Buttons */}
      {showTradingButtons && status === 'Active' && (
        <div className="grid grid-cols-2 gap-3 mb-4">
          <button
            className="trade-yes px-4 py-3 text-sm font-semibold flex items-center justify-center gap-2"
            onClick={() => handleTrade('YES')}
            disabled={loading}
          >
            Yes <LivePriceComponent className="font-semibold" />
          </button>
          <button
            className="trade-no px-4 py-3 text-sm font-semibold flex items-center justify-center gap-2"
            onClick={() => handleTrade('NO')}
            disabled={loading}
          >
            No <SimplePrice marketId={market.market_id} outcome={0} className="font-semibold" showCurrency={true} />
          </button>
        </div>
      )}

      {/* Market Info */}
      <div className="grid grid-cols-3 gap-4 text-xs text-gray-400 border-t border-gray-700 pt-4">
        <div className="text-center">
          <div className="flex items-center justify-center gap-1 mb-1">
            <Users className="w-3 h-3" />
          </div>
          <div>-- traders</div>
        </div>
        
        <div className="text-center">
          <div className="flex items-center justify-center gap-1 mb-1">
            <Clock className="w-3 h-3" />
          </div>
          <div>{formatRelativeTime(market.end_time)}</div>
        </div>
        
        <div className="text-center">
          <div className="flex items-center justify-center gap-1 mb-1">
            <Calendar className="w-3 h-3" />
          </div>
          <div>Resolves {formatRelativeTime(market.resolution_time)}</div>
        </div>
      </div>

      {/* Creator Info */}
      <div className="mt-4 pt-3 border-t border-gray-700">
        <div className="flex items-center justify-between text-xs text-gray-500">
          <span>By {market.creator.slice(0, 8)}...{market.creator.slice(-4)}</span>
          <span>{formatTime(market.created_at || Date.now().toString())}</span>
        </div>
      </div>
    </div>
  );
}