'use client';

import React, { useState } from 'react';
import { WalletProvider, WalletConnector } from '@/components/near-wallet';
import { PredictionSwipeInterface } from '@/components/prediction-swipe-interface';
import { MarketList } from '@/components/market-list';
import { CreateMarketModal } from '@/components/create-market-modal';
import { OrderbookProvider } from '@/contexts/OrderbookContext';
import { DebugTest } from '@/components/debug-test';
import { 
  Zap, 
  Users, 
  Target, 
  Search,
  Settings,
  Trophy,
  BarChart3
} from 'lucide-react';

type ViewMode = 'swipe' | 'explore' | 'leaderboard' | 'profile';

export default function FutureCastApp() {
  const [currentView, setCurrentView] = useState<ViewMode>('swipe');
  const [showCreateModal, setShowCreateModal] = useState(false);

  const renderView = () => {
    switch (currentView) {
      case 'swipe':
        return <PredictionSwipeInterface />;
      case 'explore':
        return (
          <div className="futurecast-bg min-h-screen p-4">
            <div className="max-w-4xl mx-auto">
              <div className="mb-6">
                <h1 className="text-3xl font-bold text-white mb-2">Explore Markets</h1>
                <p className="text-gray-400">Discover all active prediction markets</p>
              </div>
              <MarketList showTradingButtons={true} initialStatus="Active" />
            </div>
          </div>
        );
      case 'leaderboard':
        return <LeaderboardView />;
      case 'profile':
        return <ProfileView />;
      default:
        return <PredictionSwipeInterface />;
    }
  };

  return (
    <WalletProvider>
      <OrderbookProvider>
        <div
          className={`futurecast-bg ${
            currentView === 'swipe' ? 'h-screen overflow-hidden' : 'min-h-screen with-bottom-nav-safe-area'
          }`}
        >
        {/* Main Content */}
        {renderView()}

        {/* Bottom Navigation */}
        <div className="fixed bottom-0 left-0 right-0 z-50 bg-black/90 backdrop-blur-xl border-t border-gray-800">
          <div className="flex items-center justify-around h-20 px-4 bottom-nav-safe">
            <button
              onClick={() => setCurrentView('swipe')}
              className={`flex flex-col items-center gap-1 p-2 rounded-xl transition-all ${
                currentView === 'swipe' 
                  ? 'text-red-400 bg-red-500/20' 
                  : 'text-gray-400 hover:text-white'
              }`}
            >
              <Zap className="w-6 h-6" />
              <span className="text-xs font-medium">Swipe</span>
            </button>

            <button
              onClick={() => setCurrentView('explore')}
              className={`flex flex-col items-center gap-1 p-2 rounded-xl transition-all ${
                currentView === 'explore' 
                  ? 'text-blue-400 bg-blue-500/20' 
                  : 'text-gray-400 hover:text-white'
              }`}
            >
              <Search className="w-6 h-6" />
              <span className="text-xs font-medium">Explore</span>
            </button>

            <button
              onClick={() => setShowCreateModal(true)}
              className="w-14 h-14 bg-gradient-to-r from-red-500 to-purple-600 rounded-full flex items-center justify-center shadow-lg hover:scale-110 transition-transform"
            >
              <span className="text-2xl">+</span>
            </button>

            <button
              onClick={() => setCurrentView('leaderboard')}
              className={`flex flex-col items-center gap-1 p-2 rounded-xl transition-all ${
                currentView === 'leaderboard' 
                  ? 'text-yellow-400 bg-yellow-500/20' 
                  : 'text-gray-400 hover:text-white'
              }`}
            >
              <Trophy className="w-6 h-6" />
              <span className="text-xs font-medium">Ranks</span>
            </button>

            <button
              onClick={() => setCurrentView('profile')}
              className={`flex flex-col items-center gap-1 p-2 rounded-xl transition-all ${
                currentView === 'profile' 
                  ? 'text-green-400 bg-green-500/20' 
                  : 'text-gray-400 hover:text-white'
              }`}
            >
              <Users className="w-6 h-6" />
              <span className="text-xs font-medium">Profile</span>
            </button>
          </div>
        </div>

        {/* Create Market Modal */}
        <CreateMarketModal
          isOpen={showCreateModal}
          onClose={() => setShowCreateModal(false)}
          onMarketCreated={(marketId) => {
            console.log('Market created:', marketId);
            setCurrentView('explore');
          }}
        />
        </div>
      </OrderbookProvider>
    </WalletProvider>
  );
}

// Leaderboard Component
function LeaderboardView() {
  const topPredictors = [
    { rank: 1, name: 'CryptoOracle', streak: 15, accuracy: 89, points: 12450, avatar: '👑' },
    { rank: 2, name: 'TrendMaster', streak: 12, accuracy: 85, points: 11200, avatar: '🔥' },
    { rank: 3, name: 'FutureVision', streak: 10, accuracy: 82, points: 9800, avatar: '⚡' },
    { rank: 4, name: 'MarketSage', streak: 8, accuracy: 80, points: 8900, avatar: '🎯' },
    { rank: 5, name: 'PredictPro', streak: 7, accuracy: 78, points: 8200, avatar: '🚀' },
  ];

  const categories = [
    { name: 'Crypto', leader: 'BitcoinBull', points: 5600, icon: '₿' },
    { name: 'Politics', leader: 'VoteTracker', points: 4800, icon: '🗳️' },
    { name: 'Sports', leader: 'GameCaller', points: 4200, icon: '⚽' },
    { name: 'Tech', leader: 'InnoWatcher', points: 3900, icon: '💻' },
  ];

  return (
    <div className="futurecast-bg min-h-screen p-4 pb-24">
      <div className="max-w-4xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between mb-8">
          <div>
            <h1 className="text-3xl font-bold text-white mb-2">Leaderboard</h1>
            <p className="text-gray-400">Top predictors and rising stars</p>
          </div>
          <WalletConnector />
        </div>

        {/* Global Leaderboard */}
        <div className="futurecast-card p-6 mb-6">
          <div className="flex items-center gap-2 mb-4">
            <Trophy className="w-5 h-5 text-yellow-400" />
            <h2 className="text-xl font-bold text-white">Global Rankings</h2>
          </div>
          
          <div className="space-y-3">
            {topPredictors.map((predictor, index) => (
              <div
                key={predictor.rank}
                className={`flex items-center gap-4 p-4 rounded-xl transition-all ${
                  index < 3 
                    ? 'bg-gradient-to-r from-yellow-500/10 to-orange-500/10 border border-yellow-500/20' 
                    : 'bg-gray-800/50 hover:bg-gray-700/50'
                }`}
              >
                <div className="flex items-center gap-3">
                  <div className={`w-8 h-8 rounded-full flex items-center justify-center font-bold ${
                    predictor.rank === 1 ? 'bg-yellow-500 text-black' :
                    predictor.rank === 2 ? 'bg-gray-400 text-black' :
                    predictor.rank === 3 ? 'bg-orange-500 text-black' :
                    'bg-gray-600 text-white'
                  }`}>
                    {predictor.rank}
                  </div>
                  <div className="text-2xl">{predictor.avatar}</div>
                  <div>
                    <div className="font-semibold text-white">{predictor.name}</div>
                    <div className="text-sm text-gray-400">
                      {predictor.points.toLocaleString()} points
                    </div>
                  </div>
                </div>
                
                <div className="ml-auto flex items-center gap-6 text-sm">
                  <div className="text-center">
                    <div className="text-orange-400 font-semibold">{predictor.streak}</div>
                    <div className="text-gray-500">streak</div>
                  </div>
                  <div className="text-center">
                    <div className="text-green-400 font-semibold">{predictor.accuracy}%</div>
                    <div className="text-gray-500">accuracy</div>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>

        {/* Category Leaders */}
        <div className="futurecast-card p-6">
          <div className="flex items-center gap-2 mb-4">
            <Target className="w-5 h-5 text-blue-400" />
            <h2 className="text-xl font-bold text-white">Category Champions</h2>
          </div>
          
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            {categories.map((category) => (
              <div
                key={category.name}
                className="p-4 bg-gray-800/50 rounded-xl border border-gray-700 hover:border-gray-600 transition-colors"
              >
                <div className="flex items-center gap-3 mb-2">
                  <div className="text-2xl">{category.icon}</div>
                  <div>
                    <div className="font-semibold text-white">{category.name}</div>
                    <div className="text-sm text-gray-400">Champion</div>
                  </div>
                </div>
                <div className="flex items-center justify-between">
                  <div className="font-bold text-blue-400">{category.leader}</div>
                  <div className="text-sm text-gray-400">
                    {category.points.toLocaleString()} pts
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}

// Profile Component
function ProfileView() {
  const userStats = {
    name: 'YourUsername',
    level: 'Oracle',
    rank: 127,
    points: 2840,
    streak: 7,
    accuracy: 69,
    totalPredictions: 45,
    correctPredictions: 31,
    favoriteCategory: 'Crypto',
    joinDate: 'March 2024'
  };

  const recentPredictions = [
    { market: 'Bitcoin $100K by EOY', prediction: 'YES', result: 'pending', confidence: 72 },
    { market: 'AI Breakthrough 2024', prediction: 'YES', result: 'correct', confidence: 58 },
    { market: 'Election Winner', prediction: 'NO', result: 'incorrect', confidence: 45 },
  ];

  return (
    <div className="futurecast-bg min-h-screen p-4 pb-24">
      <div className="max-w-4xl mx-auto">
        {/* Header */}
        <div className="flex items-center justify-between mb-8">
          <div>
            <h1 className="text-3xl font-bold text-white mb-2">Your Profile</h1>
            <p className="text-gray-400">Track your prediction journey</p>
          </div>
          <button className="action-button">
            <Settings className="w-5 h-5" />
          </button>
        </div>

        {/* Profile Card */}
        <div className="futurecast-card p-6 mb-6">
          <div className="flex items-center gap-4 mb-6">
            <div className="w-16 h-16 bg-gradient-to-r from-red-500 to-purple-600 rounded-full flex items-center justify-center text-2xl">
              👤
            </div>
            <div>
              <h2 className="text-2xl font-bold text-white">{userStats.name}</h2>
              <div className="flex items-center gap-2 text-gray-400">
                <span>{userStats.level}</span>
                <span>•</span>
                <span>Rank #{userStats.rank.toLocaleString()}</span>
                <span>•</span>
                <span>{userStats.joinDate}</span>
              </div>
            </div>
          </div>

          {/* Stats Grid */}
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            <div className="text-center p-3 bg-gray-800/50 rounded-xl">
              <div className="text-2xl font-bold text-red-400">{userStats.points.toLocaleString()}</div>
              <div className="text-gray-400 text-sm">Points</div>
            </div>
            <div className="text-center p-3 bg-gray-800/50 rounded-xl">
              <div className="text-2xl font-bold text-orange-400">{userStats.streak}</div>
              <div className="text-gray-400 text-sm">Streak</div>
            </div>
            <div className="text-center p-3 bg-gray-800/50 rounded-xl">
              <div className="text-2xl font-bold text-green-400">{userStats.accuracy}%</div>
              <div className="text-gray-400 text-sm">Accuracy</div>
            </div>
            <div className="text-center p-3 bg-gray-800/50 rounded-xl">
              <div className="text-2xl font-bold text-blue-400">{userStats.totalPredictions}</div>
              <div className="text-gray-400 text-sm">Predictions</div>
            </div>
          </div>
        </div>

        {/* Recent Activity */}
        <div className="futurecast-card p-6">
          <div className="flex items-center gap-2 mb-4">
            <BarChart3 className="w-5 h-5 text-purple-400" />
            <h2 className="text-xl font-bold text-white">Recent Predictions</h2>
          </div>
          
          <div className="space-y-3">
            {recentPredictions.map((pred, index) => (
              <div
                key={index}
                className="flex items-center justify-between p-4 bg-gray-800/50 rounded-xl"
              >
                <div className="flex items-center gap-3">
                  <div className={`w-3 h-3 rounded-full ${
                    pred.result === 'pending' ? 'bg-yellow-400' :
                    pred.result === 'correct' ? 'bg-green-400' :
                    'bg-red-400'
                  }`} />
                  <div>
                    <div className="font-medium text-white">{pred.market}</div>
                    <div className="text-sm text-gray-400">
                      Predicted <span className={pred.prediction === 'YES' ? 'text-green-400' : 'text-red-400'}>
                        {pred.prediction}
                      </span>
                    </div>
                  </div>
                </div>
                <div className="text-right">
                  <div className="text-sm font-medium text-white">{pred.confidence}%</div>
                  <div className="text-xs text-gray-400">confidence</div>
                </div>
              </div>
            ))}
          </div>
        </div>
      </div>
    </div>
  );
}