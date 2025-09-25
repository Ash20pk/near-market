'use client';

import React, { useState, useEffect } from 'react';
import { useParams, useSearchParams } from 'next/navigation';
import { WalletProvider, WalletConnector } from '@/components/near-wallet';
import { TradingInterface } from '@/components/trading-interface';
import { useWallet } from '@/components/near-wallet';
import { Orderbook } from '@/components/orderbook';
import { Market } from '@/lib/near';
import { marketService, EnhancedMarket } from '@/services/market';
import { formatTime, formatRelativeTime, getMarketStatusColor, getMarketStatusText, formatCurrency, calculateProbability } from '@/lib/utils';
import { Calendar, Clock, TrendingUp, Users, Volume2, Share2, ArrowLeft, ExternalLink } from 'lucide-react';
import { MarketChart } from '@/components/market-chart';
import { DiscussionThread } from '@/components/discussion-thread';
import { Button } from '@/components/ui/button';
import Link from 'next/link';
import { OrderbookProvider } from '@/contexts/OrderbookContext';
import {
  LivePriceDisplay,
  SimplePrice,
  PriceChange
} from '@/components/orderbook/LivePriceDisplay';
import { useMarketPrice } from '@/hooks/useOrderbook';
import { OrderbookWidget } from '@/components/orderbook/OrderbookWidget';
import { TradeHistory } from '@/components/orderbook/TradeHistory';
import { OrderbookChart } from '@/components/orderbook/OrderbookChart';

// Component to conditionally show price label only when there's trading data
function PriceLabelWithData({ marketId, outcome }: { marketId: string; outcome: number }) {
  const { price } = useMarketPrice(marketId, outcome);

  // Only show the label if we have actual price data
  if (!price) {
    return null;
  }

  return (
    <div className="text-sm text-gray-400 flex items-center justify-center gap-2">
      YES probability
      <PriceChange marketId={marketId} outcome={outcome} className="text-xs" />
    </div>
  );
}

// Component to show market activity or "no data" message
function MarketActivitySection({ marketId }: { marketId: string }) {
  const { price, loading } = useMarketPrice(marketId, 1);

  // If we've finished loading and there's no price data, show "no data" message
  const hasNoData = !loading && !price;

  if (hasNoData) {
    return (
      <div className="futurecast-card p-6">
        <div className="flex items-center justify-between mb-4">
          <h2 className="text-xl font-bold text-white">Market Activity</h2>
        </div>

        <div className="text-center py-12">
          <div className="text-gray-400 mb-2 text-4xl">📊</div>
          <h3 className="text-lg font-semibold text-gray-300 mb-2">No Market Data Yet</h3>
          <p className="text-gray-400 text-sm">
            This market hasn't been traded yet. Be the first to place an order!
          </p>
        </div>
      </div>
    );
  }

  // Show the full market activity section with orderbook and trades
  return (
    <div className="futurecast-card p-6">
      <div className="flex items-center justify-between mb-4">
        <h2 className="text-xl font-bold text-white">Market Activity</h2>
        <span className="text-xs text-gray-400">Live orderbook & recent trades</span>
      </div>

      {/* Orderbook Widget */}
      <div className="mb-6">
        <h3 className="text-sm font-semibold text-gray-300 mb-3">Current Orders</h3>
        <OrderbookWidget
          marketId={marketId}
          outcome={1}
          maxLevels={8}
          showHeader={false}
          className="mb-4"
        />
      </div>

      {/* Trade History */}
      <div>
        <h3 className="text-sm font-semibold text-gray-300 mb-3">Recent Trades</h3>
        <TradeHistory
          marketId={marketId}
          maxTrades={10}
          showHeader={false}
        />
      </div>
    </div>
  );
}

function MarketDetailContent() {
  const params = useParams();
  const searchParams = useSearchParams();
  const { nearService } = useWallet();
  const [market, setMarket] = useState<EnhancedMarket | null>(null);
  const [loading, setLoading] = useState(true);
  const [tradeResult, setTradeResult] = useState<any>(null);
  
  const marketId = params.id as string;
  const initialTrade = searchParams.get('trade') as 'YES' | 'NO' | null;

  useEffect(() => {
    loadMarket();
  }, [marketId]);

  const loadMarket = async () => {
    console.log(`[MarketDetail] Loading market details for: ${marketId}`);
    setLoading(true);
    try {
      const fetchedMarket = await marketService.getMarket(marketId);
      if (fetchedMarket) {
        console.log(`[MarketDetail] ✅ Successfully loaded market: ${fetchedMarket.title}`);
        console.log(`[MarketDetail] Market data:`, {
          id: fetchedMarket.market_id,
          title: fetchedMarket.title,
          category: fetchedMarket.category,
          is_active: fetchedMarket.is_active,
          end_time: fetchedMarket.end_time
        });
      } else {
        console.log(`[MarketDetail] ❌ Market not found: ${marketId}`);
      }
      setMarket(fetchedMarket); // Can be null if market doesn't exist
    } catch (error) {
      console.error(`[MarketDetail] ❌ Error loading market ${marketId}:`, error);
      setMarket(null);
    } finally {
      setLoading(false);
    }
  };


  if (loading) {
    return (
      <div className="futurecast-bg min-h-screen flex items-center justify-center">
        <div className="text-center">
          <div className="animate-pulse">
            <div className="h-8 bg-gray-600 rounded w-64 mb-4"></div>
            <div className="h-4 bg-gray-600 rounded w-48"></div>
          </div>
        </div>
      </div>
    );
  }

  if (!market) {
    return (
      <div className="futurecast-bg min-h-screen flex items-center justify-center">
        <div className="text-center">
          <h2 className="text-2xl font-bold text-white mb-4">Market Not Found</h2>
          <p className="text-gray-400 mb-6">The requested market could not be found.</p>
          <Link href="/">
            <Button>
              <ArrowLeft className="w-4 h-4 mr-2" />
              Back to Markets
            </Button>
          </Link>
        </div>
      </div>
    );
  }

  const status = getMarketStatusText(market.is_active, market.end_time);
  const volume = market.total_volume || '0';

  return (
    <div className="futurecast-bg min-h-screen">
      {/* Mobile-First Header */}
      <header className="bg-black/50 backdrop-blur-xl border-b border-gray-800 sticky top-0 z-40">
        <div className="container mx-auto px-4 py-3">
          <div className="flex items-center justify-between">
            <Link href="/">
              <Button variant="ghost" size="sm" className="text-white hover:text-red-400 p-2">
                <ArrowLeft className="w-5 h-5" />
                <span className="ml-2 hidden sm:inline">Back</span>
              </Button>
            </Link>

            <div className="flex items-center gap-2">
              {/* Show new market indicator if no trading data */}
              {!market.current_price && (
                <span className="bg-blue-500/20 text-blue-400 px-3 py-1 rounded-full text-xs font-medium">
                  New Market
                </span>
              )}
              <WalletConnector />
              <Button variant="ghost" size="sm" className="text-gray-300 hover:text-white p-2">
                <Share2 className="w-5 h-5" />
              </Button>
            </div>
          </div>
        </div>
      </header>

      {/* Main Content - Mobile First */}
      <div className="container mx-auto px-4 py-6 space-y-6 pb-20">

        {/* Market Header */}
        <div className="futurecast-card p-6">
          <div className="text-center mb-6">
            {/* Live Price Display */}
            <div className="text-4xl sm:text-5xl font-bold text-green-400 mb-2">
              <LivePriceDisplay
                marketId={market.market_id}
                outcome={1}
                className="text-4xl sm:text-5xl font-bold text-green-400"
                showCurrency={true}
                fallback={<span className="text-gray-400">No Data</span>}
              />
            </div>
            {/* Only show "YES probability" label when there's actual price data */}
            <PriceLabelWithData marketId={market.market_id} outcome={1} />
          </div>

          {/* Title */}
          <h1 className="text-xl sm:text-2xl font-bold text-white mb-4 text-center leading-tight">
            {market.title}
          </h1>

          {/* Tags */}
          <div className="flex flex-wrap justify-center gap-2 mb-6">
            <span className="bg-purple-500/20 text-purple-400 px-3 py-1 rounded-full text-xs font-medium">
              {market.category}
            </span>
            <span className={`px-3 py-1 rounded-full text-xs font-medium ${
              status === 'Active' ? 'bg-green-500/20 text-green-400' :
              'bg-yellow-500/20 text-yellow-400'
            }`}>
              {status}
            </span>
          </div>

          {/* Description */}
          <p className="text-gray-300 text-sm sm:text-base leading-relaxed text-center mb-6">
            {market.description}
          </p>

          {/* Key Stats Grid */}
          <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
            <div className="text-center p-4 bg-gray-800/30 rounded-xl">
              <div className="flex items-center justify-center gap-2 mb-2">
                <Volume2 className="w-4 h-4 text-blue-400" />
                <span className="text-sm text-gray-400">Volume</span>
              </div>
              <div className="text-white font-bold text-lg">
                {market.total_volume ? formatCurrency(parseFloat(market.total_volume) / 1e6) : 'No trades yet'}
              </div>
            </div>

            <div className="text-center p-4 bg-gray-800/30 rounded-xl">
              <div className="flex items-center justify-center gap-2 mb-2">
                <Clock className="w-4 h-4 text-orange-400" />
                <span className="text-sm text-gray-400">Ends</span>
              </div>
              <div className="text-white font-bold text-sm">
                {formatRelativeTime(market.end_time)}
              </div>
            </div>
          </div>
        </div>

        {/* Market Overview - Current Prices */}
        <div className="futurecast-card p-6">
          <h2 className="text-xl font-bold text-white mb-4">Current Prices</h2>
          <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
            <div className="p-4 bg-green-500/10 border border-green-500/20 rounded-xl">
              <div className="flex items-center justify-between">
                <span className="text-green-400 font-semibold">YES shares</span>
                <div className="text-right">
                  <SimplePrice
                    marketId={market.market_id}
                    outcome={1}
                    className="text-green-400 font-bold text-lg"
                    showCurrency={true}
                    fallback={<span className="text-gray-400">No data</span>}
                  />
                  <PriceChange marketId={market.market_id} outcome={1} className="text-xs block mt-1" />
                </div>
              </div>
            </div>

            <div className="p-4 bg-red-500/10 border border-red-500/20 rounded-xl">
              <div className="flex items-center justify-between">
                <span className="text-red-400 font-semibold">NO shares</span>
                <div className="text-right">
                  <SimplePrice
                    marketId={market.market_id}
                    outcome={0}
                    className="text-red-400 font-bold text-lg"
                    showCurrency={true}
                    fallback={<span className="text-gray-400">No data</span>}
                  />
                  <PriceChange marketId={market.market_id} outcome={0} className="text-xs block mt-1" />
                </div>
              </div>
            </div>
          </div>
        </div>

        {/* Trading Interface */}
        <TradingInterface
          market={market}
          onTradeSubmitted={(result) => {
            setTradeResult(result);
            loadMarket();
          }}
        />

        {/* Trade Result Alert */}
        {tradeResult && (
          <div className="futurecast-card p-4 border-green-500/30 bg-green-500/10">
            <h3 className="text-lg font-bold text-green-400 mb-2">✅ Trade Executed</h3>
            <p className="text-green-300 text-sm">{tradeResult.execution_details}</p>
            <p className="text-xs text-gray-400 mt-1">Intent ID: {tradeResult.intent_id}</p>
          </div>
        )}

        {/* Market Activity - Smart loading and empty state */}
        <MarketActivitySection marketId={market.market_id} />

        {/* Market Details */}
        <div className="futurecast-card p-6">
          <h2 className="text-xl font-bold text-white mb-4">Market Details</h2>
          <div className="space-y-3">
            <div className="flex flex-col sm:flex-row sm:justify-between sm:items-center gap-2">
              <span className="text-gray-400 font-medium">Creator</span>
              <span className="text-white font-mono text-sm break-all">{market.creator}</span>
            </div>
            <div className="flex flex-col sm:flex-row sm:justify-between sm:items-center gap-2">
              <span className="text-gray-400 font-medium">Resolver</span>
              <span className="text-white font-mono text-sm break-all">{market.resolver}</span>
            </div>
            <div className="flex flex-col sm:flex-row sm:justify-between sm:items-center gap-2">
              <span className="text-gray-400 font-medium">Resolution Time</span>
              <span className="text-white">{formatRelativeTime(market.resolution_time)}</span>
            </div>
            <div className="flex flex-col sm:flex-row sm:justify-between sm:items-center gap-2">
              <span className="text-gray-400 font-medium">Market ID</span>
              <span className="text-gray-300 font-mono text-xs break-all">{market.market_id}</span>
            </div>
          </div>
        </div>

      </div>
    </div>
  );
}

export default function MarketDetailPage() {
  return (
    <WalletProvider>
      <OrderbookProvider>
        <MarketDetailContent />
      </OrderbookProvider>
    </WalletProvider>
  );
}