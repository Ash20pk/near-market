'use client';

import React, { useState, useEffect } from 'react';
import { useParams, useSearchParams } from 'next/navigation';
import { WalletProvider } from '@/components/near-wallet';
import { TradingInterface } from '@/components/trading-interface';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { useWallet } from '@/components/near-wallet';
import { Orderbook } from '@/components/orderbook';
import { Market } from '@/lib/near';
import { formatTime, formatRelativeTime, getMarketStatusColor, getMarketStatusText, formatCurrency, calculateProbability } from '@/lib/utils';
import { Calendar, Clock, TrendingUp, Users, Volume2, Share2, ArrowLeft, ExternalLink } from 'lucide-react';
import Link from 'next/link';

function MarketDetailContent() {
  const params = useParams();
  const searchParams = useSearchParams();
  const { nearService } = useWallet();
  const [market, setMarket] = useState<Market | null>(null);
  const [loading, setLoading] = useState(true);
  const [tradeResult, setTradeResult] = useState<any>(null);
  
  const marketId = params.id as string;
  const initialTrade = searchParams.get('trade') as 'YES' | 'NO' | null;

  useEffect(() => {
    loadMarket();
  }, [marketId]);

  const loadMarket = async () => {
    setLoading(true);
    try {
      const fetchedMarket = await nearService.getMarket(marketId);
      if (fetchedMarket) {
        setMarket(fetchedMarket);
      } else {
        // Generate demo market if not found
        setMarket(generateDemoMarket(marketId));
      }
    } catch (error) {
      console.error('Error loading market:', error);
      setMarket(generateDemoMarket(marketId));
    } finally {
      setLoading(false);
    }
  };

  const generateDemoMarket = (id: string): Market => {
    const markets = {
      'btc-100k-2024': {
        market_id: 'btc-100k-2024',
        condition_id: 'cond_btc_100k',
        title: 'Will Bitcoin reach $100,000 by end of 2024?',
        description: 'This market resolves to "Yes" if Bitcoin (BTC) reaches or exceeds $100,000 USD on any major exchange (Coinbase, Binance, Kraken) by December 31, 2024, 11:59 PM UTC. The price must be sustained for at least 1 hour to count as a valid resolution.',
        creator: 'oracle.testnet',
        end_time: String(Date.now() * 1000000 + 30 * 24 * 60 * 60 * 1000 * 1000000),
        resolution_time: String(Date.now() * 1000000 + 35 * 24 * 60 * 60 * 1000 * 1000000),
        category: 'Crypto',
        is_active: true,
        resolver: 'resolver.testnet',
        total_volume: '45000000000',
        created_at: String(Date.now() - 7 * 24 * 60 * 60 * 1000)
      }
    };
    
    return markets[id as keyof typeof markets] || {
      market_id: id,
      condition_id: `cond_${id}`,
      title: `Market ${id}`,
      description: 'Demo market description',
      creator: 'demo.testnet',
      end_time: String(Date.now() * 1000000 + 30 * 24 * 60 * 60 * 1000 * 1000000),
      resolution_time: String(Date.now() * 1000000 + 35 * 24 * 60 * 60 * 1000 * 1000000),
      category: 'Other',
      is_active: true,
      resolver: 'resolver.testnet'
    };
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
  const statusColor = getMarketStatusColor(market.is_active, market.end_time);
  
  // Mock trading data
  const yesShares = Math.floor(Math.random() * 1000) + 500;
  const noShares = Math.floor(Math.random() * 1000) + 500;
  const probability = calculateProbability(yesShares, noShares);
  const volume = market.total_volume || (Math.floor(Math.random() * 50000) + 10000).toString();

  return (
    <div className="futurecast-bg min-h-screen with-bottom-nav-safe-area">
      {/* Header */}
      <header className="bg-black/50 backdrop-blur-xl border-b border-gray-800 sticky top-0 z-40">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex items-center justify-between h-16">
            <div className="flex items-center gap-4">
              <Link href="/">
                <Button variant="ghost" size="sm" className="text-white hover:text-red-400">
                  <ArrowLeft className="w-4 h-4 mr-2" />
                  Back
                </Button>
              </Link>
              <div className="h-6 w-px bg-gray-600" />
              <div className="hidden sm:block">
                <h1 className="text-lg font-semibold text-white">Market Details</h1>
                <p className="text-sm text-gray-400">Trade on prediction outcomes</p>
              </div>
            </div>
            
            <div className="flex items-center gap-3">
              <Button variant="outline" size="sm" className="hidden sm:flex border-gray-600 text-gray-300 hover:text-white hover:border-gray-500">
                <Share2 className="w-4 h-4 mr-2" />
                Share
              </Button>
              <Button variant="outline" size="sm" className="hidden md:flex border-gray-600 text-gray-300 hover:text-white hover:border-gray-500">
                <ExternalLink className="w-4 h-4 mr-2" />
                Explorer
              </Button>
            </div>
          </div>
        </div>
      </header>

      {/* Mobile-first responsive container */}
      <div className="w-full">
        {/* Mobile: Stack everything vertically */}
        <div className="block lg:hidden px-4 py-4 space-y-4">
          {/* Mobile Market Card */}
          <div className="futurecast-card p-4">
            {/* Main Market Card */}
            <div className="futurecast-card p-4 md:p-6">
              <div className="flex flex-col sm:flex-row sm:items-start justify-between gap-6">
                <div className="flex-1">
                  <h1 className="text-2xl sm:text-3xl font-bold text-white mb-4 leading-tight">{market.title}</h1>
                  <div className="flex flex-wrap gap-2 mb-6">
                    <span className="bg-red-500/20 text-red-400 px-3 py-1 rounded-full text-sm font-medium">
                      {market.category}
                    </span>
                    <span className={`px-3 py-1 rounded-full text-sm font-medium ${
                      status === 'Active' ? 'bg-green-500/20 text-green-400' : 
                      status === 'Closed' ? 'bg-yellow-500/20 text-yellow-400' : 
                      'bg-red-500/20 text-red-400'
                    }`}>
                      {status}
                    </span>
                  </div>
                </div>
                
                <div className="text-center sm:text-right">
                  <div className="text-4xl sm:text-5xl font-bold text-green-400 mb-2">
                    {(probability * 100).toFixed(0)}%
                  </div>
                  <div className="text-sm text-gray-400">Yes probability</div>
                </div>
              </div>

              <div className="space-y-6">
                <p className="text-gray-200 leading-relaxed text-lg">{market.description}</p>
                
                {/* Market Stats */}
                <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 md:gap-4 pt-6 border-t border-gray-700">
                  <div className="text-center">
                    <div className="flex items-center justify-center gap-1 text-sm text-gray-400 mb-1">
                      <Volume2 className="w-4 h-4" />
                      Volume
                    </div>
                    <div className="font-semibold text-white">{formatCurrency(parseFloat(volume) / 1e6)}</div>
                  </div>
                  <div className="text-center">
                    <div className="flex items-center justify-center gap-1 text-sm text-gray-400 mb-1">
                      <Users className="w-4 h-4" />
                      Traders
                    </div>
                    <div className="font-semibold text-white">{Math.floor(Math.random() * 50) + 10}</div>
                  </div>
                  <div className="text-center">
                    <div className="flex items-center justify-center gap-1 text-sm text-gray-400 mb-1">
                      <Clock className="w-4 h-4" />
                      Ends
                    </div>
                    <div className="font-semibold text-white text-sm">{formatRelativeTime(market.end_time)}</div>
                  </div>
                  <div className="text-center">
                    <div className="flex items-center justify-center gap-1 text-sm text-gray-400 mb-1">
                      <Calendar className="w-4 h-4" />
                      Resolves
                    </div>
                    <div className="font-semibold text-white text-sm">{formatRelativeTime(market.resolution_time)}</div>
                  </div>
                </div>

                {/* Market Details */}
                <div className="space-y-3 text-sm">
                  <div className="flex justify-between">
                    <span className="text-gray-400">Creator:</span>
                    <span className="font-medium text-white">{market.creator}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-400">Resolver:</span>
                    <span className="font-medium text-white">{market.resolver}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-400">Market ID:</span>
                    <span className="font-mono text-xs text-gray-300">{market.market_id}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-400">Condition ID:</span>
                    <span className="font-mono text-xs text-gray-300">{market.condition_id}</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-gray-400">Created:</span>
                    <span className="text-white">{formatTime(market.created_at || Date.now().toString())}</span>
                  </div>
                </div>
              </div>
            </div>

            {/* Price Chart Placeholder */}
            <div className="futurecast-card p-4 md:p-6">
              <h3 className="text-xl font-bold text-white mb-4">Price History</h3>
              <div className="h-64 bg-gray-800/50 rounded-lg flex items-center justify-center border border-gray-700">
                <div className="text-center text-gray-400">
                  <TrendingUp className="w-8 h-8 mx-auto mb-2" />
                  <p className="text-white">Price chart coming soon</p>
                  <p className="text-sm">Real-time market data and historical price charts</p>
                </div>
              </div>
            </div>

            {/* Trade Result */}
            {tradeResult && (
              <div className="futurecast-card p-6 border-green-500/30 bg-green-500/10">
                <h3 className="text-xl font-bold text-green-400 mb-4">Trade Executed</h3>
                <div className="text-green-300">
                  <p className="font-medium">{tradeResult.execution_details}</p>
                  <p className="text-sm mt-2 text-gray-400">Intent ID: {tradeResult.intent_id}</p>
                </div>
              </div>
            )}
          </div>

          {/* Trading Interface */}
          <div className="space-y-4 md:space-y-6">
            <TradingInterface
              market={market}
              onTradeSubmitted={(result) => {
                setTradeResult(result);
                loadMarket(); // Refresh market data
              }}
            />

            {/* Orderbook */}
            <div className="futurecast-card p-4 md:p-6">
              <h3 className="text-lg font-bold text-white mb-4">Orderbook</h3>
              <Orderbook marketId={market.market_id} />
            </div>

            {/* Quick Stats */}
            <div className="futurecast-card p-4 md:p-6">
              <h3 className="text-lg font-bold text-white mb-4">Market Overview</h3>
              <div className="space-y-3">
                <div className="flex justify-between">
                  <span className="text-sm text-gray-400">Yes Price</span>
                  <span className="font-medium text-green-400">${(probability * 100).toFixed(2)}¢</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-sm text-gray-400">No Price</span>
                  <span className="font-medium text-red-400">${((1 - probability) * 100).toFixed(2)}¢</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-sm text-gray-400">Spread</span>
                  <span className="font-medium text-white">2.5¢</span>
                </div>
              </div>
            </div>

            {/* Market Rules */}
            <div className="futurecast-card p-4 md:p-6">
              <h3 className="text-lg font-bold text-white mb-4">Resolution Rules</h3>
              <div className="space-y-2 text-sm text-gray-300">
                <p>• Market resolves based on official sources</p>
                <p>• Resolution can be disputed within 24 hours</p>
                <p>• Invalid markets result in full refunds</p>
                <p>• Trading fees: 1% of transaction volume</p>
              </div>
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
      <MarketDetailContent />
    </WalletProvider>
  );
}