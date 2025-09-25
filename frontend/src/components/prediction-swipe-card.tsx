'use client';

import React, { useState, useEffect } from 'react';
import { Market } from '@/lib/near';
import { formatRelativeTime, formatCurrency, calculateProbability } from '@/lib/utils';
import { 
  Heart, 
  MessageCircle, 
  Share2, 
  Users, 
  Clock, 
  TrendingUp, 
  TrendingDown,
  Zap,
  Target,
  Crown,
  Flame
} from 'lucide-react';
import Image from 'next/image';

interface PredictionSwipeCardProps {
  market: Market;
  onSwipe: (direction: 'yes' | 'no' | 'skip') => void;
  currentStreak: number;
  userLevel: string;
  socialStats: {
    likes: number;
    comments: number;
    shares: number;
    participants: number;
  };
}

export function PredictionSwipeCard({ 
  market, 
  onSwipe, 
  currentStreak,
  userLevel,
  socialStats 
}: PredictionSwipeCardProps) {
  const [confidence, setConfidence] = useState(0.72); // Mock confidence level
  const [isAnimating, setIsAnimating] = useState(false);
  const [showReasons, setShowReasons] = useState(false);

  // Mock data for demo
  const backgroundImage = `/api/placeholder/400/800`;
  const probability = calculateProbability(750, 450);
  
  const reasons = {
    yes: [
      "Historical trends support this outcome",
      "Recent market indicators are positive", 
      "Expert predictions align with YES",
      "Community sentiment is 68% bullish"
    ],
    no: [
      "Regulatory uncertainty could impact this",
      "Market volatility remains high",
      "Past similar events had different outcomes",
      "Economic indicators suggest otherwise"
    ]
  };

  const handleSwipeAction = (direction: 'yes' | 'no' | 'skip') => {
    setIsAnimating(true);
    
    // Haptic feedback simulation
    if (navigator.vibrate) {
      navigator.vibrate(direction === 'skip' ? 50 : 100);
    }
    
    setTimeout(() => {
      onSwipe(direction);
      setIsAnimating(false);
    }, 300);
  };

  const formatMarketTitle = (title: string) => {
    // Convert boring market titles to engaging stories
    if (title.includes('Bitcoin') && title.includes('100,000')) {
      return "🚀 Will Bitcoin finally break the legendary $100K barrier?";
    }
    if (title.includes('Election') || title.includes('President')) {
      return "🗳️ The race that could change everything - who wins?";
    }
    if (title.includes('AI')) {
      return "🤖 Will AI achieve another mind-blowing breakthrough?";
    }
    return title;
  };

  const getStoryDescription = (description: string) => {
    // Convert technical descriptions to story format
    if (description.includes('Bitcoin')) {
      return "The crypto community has been waiting for this moment for years. With institutional adoption growing and ETFs approved, could this finally be Bitcoin's time to shine? 💎";
    }
    if (description.includes('Election')) {
      return "Political tensions are at an all-time high. Polls are shifting daily, and anything could happen. Your prediction could be worth serious bragging rights! 📊";
    }
    return description.slice(0, 150) + "...";
  };

  return (
    <div className={`prediction-card ${isAnimating ? 'scale-in' : ''}`}>
      {/* Background Image/Video */}
      <div 
        className="prediction-card-bg"
        style={{
          backgroundImage: `url(${backgroundImage})`,
          backgroundSize: 'cover',
          backgroundPosition: 'center'
        }}
      />
      
      {/* Overlay */}
      <div className="prediction-overlay" />

      {/* Content */}
      <div className="prediction-content">
        {/* Top Section - User Stats */}
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="streak-counter flex items-center gap-2">
              <Flame className="w-4 h-4" />
              {currentStreak} streak
            </div>
            <div className="level-badge flex items-center gap-1">
              <Crown className="w-3 h-3" />
              {userLevel}
            </div>
          </div>
          
          <div className="flex items-center gap-2">
            <div className="prediction-score">
              {Math.floor(probability * 100)}%
            </div>
          </div>
        </div>

        {/* Middle Section - Story Content */}
        <div className="flex-1 flex flex-col justify-center">
          <h1 className="story-title">
            {formatMarketTitle(market.title)}
          </h1>
          
          <p className="story-description">
            {getStoryDescription(market.description)}
          </p>

          <div className="story-meta">
            <span className="flex items-center gap-1">
              <Clock className="w-4 h-4" />
              {formatRelativeTime(market.end_time)}
            </span>
            <span className="flex items-center gap-1">
              <Target className="w-4 h-4" />
              {market.category}
            </span>
            <span className="flex items-center gap-1">
              <Users className="w-4 h-4" />
              {socialStats.participants}
            </span>
          </div>

          {/* Confidence Meter */}
          <div className="mb-6">
            <div className="flex justify-between text-sm text-gray-400 mb-2">
              <span>Community thinks YES</span>
              <span>{Math.floor(confidence * 100)}%</span>
            </div>
            <div className="confidence-meter">
              <div 
                className="confidence-fill confidence-yes"
                style={{ width: `${confidence * 100}%` }}
              />
            </div>
          </div>

          {/* Quick Reasons (Optional) */}
          {showReasons && (
            <div className="bg-black/50 rounded-2xl p-4 mb-4 slide-up">
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <h4 className="text-green-400 font-semibold mb-2 flex items-center gap-1">
                    <TrendingUp className="w-4 h-4" />
                    Why YES
                  </h4>
                  <ul className="text-sm space-y-1">
                    {reasons.yes.slice(0, 2).map((reason, i) => (
                      <li key={i} className="text-gray-300">• {reason}</li>
                    ))}
                  </ul>
                </div>
                <div>
                  <h4 className="text-red-400 font-semibold mb-2 flex items-center gap-1">
                    <TrendingDown className="w-4 h-4" />
                    Why NO
                  </h4>
                  <ul className="text-sm space-y-1">
                    {reasons.no.slice(0, 2).map((reason, i) => (
                      <li key={i} className="text-gray-300">• {reason}</li>
                    ))}
                  </ul>
                </div>
              </div>
            </div>
          )}
        </div>

        {/* Bottom Section - Actions */}
        <div className="bottom-actions">
          {/* Social Stats */}
          <div className="social-stats mb-4">
            <div className="social-stat">
              <Heart className="w-4 h-4" />
              <span>{socialStats.likes.toLocaleString()}</span>
            </div>
            <div className="social-stat">
              <MessageCircle className="w-4 h-4" />
              <span>{socialStats.comments}</span>
            </div>
            <div className="social-stat">
              <Share2 className="w-4 h-4" />
              <span>{socialStats.shares}</span>
            </div>
            <div className="social-stat ml-auto">
              <span className="text-xs">
                💰 Pot: {formatCurrency(parseFloat(market.total_volume || '50000') / 1e6)}
              </span>
            </div>
          </div>

          {/* Prediction Options */}
          <div className="prediction-options">
            <button
              className="swipe-yes haptic-medium flex items-center justify-center gap-2"
              onClick={() => handleSwipeAction('yes')}
              disabled={isAnimating}
            >
              <TrendingUp className="w-5 h-5" />
              YES {Math.floor(probability * 100)}¢
            </button>
            
            <button
              className="swipe-no haptic-medium flex items-center justify-center gap-2"
              onClick={() => handleSwipeAction('no')}
              disabled={isAnimating}
            >
              <TrendingDown className="w-5 h-5" />
              NO {Math.floor((1 - probability) * 100)}¢
            </button>
          </div>

          {/* Secondary Actions */}
          <div className="flex items-center justify-between">
            <button
              className="swipe-skip haptic-light flex items-center gap-2"
              onClick={() => handleSwipeAction('skip')}
            >
              Skip this one
            </button>
            
            <button
              className="action-button haptic-light"
              onClick={() => setShowReasons(!showReasons)}
            >
              <Zap className="w-5 h-5" />
            </button>
          </div>
        </div>
      </div>

      {/* TikTok-style Action Sidebar */}
      <div className="action-sidebar">
        <button className="action-button haptic-light">
          <Heart className="w-5 h-5" />
        </button>
        <button className="action-button haptic-light">
          <MessageCircle className="w-5 h-5" />
        </button>
        <button className="action-button haptic-light">
          <Share2 className="w-5 h-5" />
        </button>
        <button 
          className={`action-button haptic-light ${showReasons ? 'active' : ''}`}
          onClick={() => setShowReasons(!showReasons)}
        >
          <Zap className="w-5 h-5" />
        </button>
      </div>
    </div>
  );
}