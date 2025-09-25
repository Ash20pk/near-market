'use client';

import React, { useState } from 'react';
import { MessageCircle, Heart, Reply, MoreVertical, Send, User } from 'lucide-react';
import { Button } from '@/components/ui/button';

interface Comment {
  id: string;
  author: string;
  content: string;
  timestamp: Date;
  likes: number;
  isLiked: boolean;
  position?: 'YES' | 'NO';
  replies?: Comment[];
}

interface DiscussionThreadProps {
  marketId: string;
}

// Mock comments data
const generateMockComments = (): Comment[] => [
  {
    id: '1',
    author: 'CryptoOracle.near',
    content: 'The technical analysis looks bullish. RSI is oversold and we\'re seeing strong support at $95k. I think we break through $100k by December.',
    timestamp: new Date(Date.now() - 2 * 60 * 60 * 1000), // 2 hours ago
    likes: 24,
    isLiked: false,
    position: 'YES',
    replies: [
      {
        id: '1-1',
        author: 'bear-market.near',
        content: 'But what about the regulatory headwinds? SEC is still cracking down.',
        timestamp: new Date(Date.now() - 1.5 * 60 * 60 * 1000),
        likes: 8,
        isLiked: false,
        position: 'NO'
      }
    ]
  },
  {
    id: '2',
    author: 'DefiDegen.near',
    content: 'Just bought more YES tokens. The institutional flow is insane right now. Blackrock and Fidelity are accumulating heavy.',
    timestamp: new Date(Date.now() - 4 * 60 * 60 * 1000),
    likes: 18,
    isLiked: true,
    position: 'YES'
  },
  {
    id: '3',
    author: 'macro-analyst.near',
    content: 'Fed policy and macro conditions don\'t support this rally. We\'re due for a correction. $100k is a pipe dream this cycle.',
    timestamp: new Date(Date.now() - 6 * 60 * 60 * 1000),
    likes: 12,
    isLiked: false,
    position: 'NO'
  },
  {
    id: '4',
    author: 'hodler4life.near',
    content: 'Been holding since $3k. This feels different. The narrative has shifted completely. $100k is just the beginning! 🚀',
    timestamp: new Date(Date.now() - 8 * 60 * 60 * 1000),
    likes: 31,
    isLiked: false,
    position: 'YES'
  }
];

export function DiscussionThread({ marketId }: DiscussionThreadProps) {
  const [comments, setComments] = useState<Comment[]>(generateMockComments());
  const [newComment, setNewComment] = useState('');
  const [replyingTo, setReplyingTo] = useState<string | null>(null);
  const [sortBy, setSortBy] = useState<'recent' | 'popular'>('recent');

  const formatTimeAgo = (date: Date) => {
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    const hours = Math.floor(diff / (1000 * 60 * 60));
    const days = Math.floor(hours / 24);
    
    if (days > 0) return `${days}d ago`;
    if (hours > 0) return `${hours}h ago`;
    return 'Just now';
  };

  const handleLike = (commentId: string) => {
    setComments(prev => prev.map(comment => {
      if (comment.id === commentId) {
        return {
          ...comment,
          isLiked: !comment.isLiked,
          likes: comment.isLiked ? comment.likes - 1 : comment.likes + 1
        };
      }
      return comment;
    }));
  };

  const handleSubmitComment = () => {
    if (!newComment.trim()) return;
    
    const comment: Comment = {
      id: Date.now().toString(),
      author: 'you.near',
      content: newComment,
      timestamp: new Date(),
      likes: 0,
      isLiked: false
    };
    
    setComments(prev => [comment, ...prev]);
    setNewComment('');
  };

  const sortedComments = [...comments].sort((a, b) => {
    if (sortBy === 'popular') {
      return b.likes - a.likes;
    }
    return b.timestamp.getTime() - a.timestamp.getTime();
  });

  return (
    <div className="futurecast-card p-4">
      {/* Header */}
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-2">
          <MessageCircle className="w-5 h-5 text-blue-400" />
          <h3 className="text-lg font-bold text-white">Discussion</h3>
          <span className="bg-gray-700 text-gray-300 px-2 py-1 rounded-full text-xs">
            {comments.length}
          </span>
        </div>
        
        <div className="flex bg-gray-800/50 rounded-lg p-1">
          <button
            onClick={() => setSortBy('recent')}
            className={`px-2 py-1 rounded text-xs font-medium transition-colors ${
              sortBy === 'recent'
                ? 'bg-red-500 text-white'
                : 'text-gray-400 hover:text-white'
            }`}
          >
            Recent
          </button>
          <button
            onClick={() => setSortBy('popular')}
            className={`px-2 py-1 rounded text-xs font-medium transition-colors ${
              sortBy === 'popular'
                ? 'bg-red-500 text-white'
                : 'text-gray-400 hover:text-white'
            }`}
          >
            Popular
          </button>
        </div>
      </div>

      {/* Comment Input */}
      <div className="mb-6">
        <div className="flex gap-3">
          <div className="w-8 h-8 bg-gradient-to-r from-red-500 to-purple-600 rounded-full flex items-center justify-center flex-shrink-0">
            <User className="w-4 h-4 text-white" />
          </div>
          <div className="flex-1">
            <textarea
              value={newComment}
              onChange={(e) => setNewComment(e.target.value)}
              placeholder="Share your analysis or prediction..."
              className="w-full bg-gray-800/50 border border-gray-600 rounded-lg px-3 py-2 text-white text-sm resize-none focus:outline-none focus:border-red-500"
              rows={3}
            />
            <div className="flex justify-end mt-2">
              <Button
                size="sm"
                onClick={handleSubmitComment}
                disabled={!newComment.trim()}
                className="bg-red-500 hover:bg-red-600 text-white px-4 py-1 text-xs"
              >
                <Send className="w-3 h-3 mr-1" />
                Post
              </Button>
            </div>
          </div>
        </div>
      </div>

      {/* Comments */}
      <div className="space-y-4">
        {sortedComments.map((comment) => (
          <div key={comment.id} className="group">
            <div className="flex gap-3">
              {/* Avatar */}
              <div className="w-8 h-8 bg-gray-700 rounded-full flex items-center justify-center flex-shrink-0">
                <User className="w-4 h-4 text-gray-400" />
              </div>
              
              {/* Comment Content */}
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2 mb-1">
                  <span className="font-medium text-white text-sm">{comment.author}</span>
                  {comment.position && (
                    <span className={`px-2 py-0.5 rounded-full text-xs font-medium ${
                      comment.position === 'YES' 
                        ? 'bg-green-500/20 text-green-400' 
                        : 'bg-red-500/20 text-red-400'
                    }`}>
                      {comment.position}
                    </span>
                  )}
                  <span className="text-gray-500 text-xs">{formatTimeAgo(comment.timestamp)}</span>
                </div>
                
                <p className="text-gray-200 text-sm leading-relaxed mb-2">
                  {comment.content}
                </p>
                
                {/* Comment Actions */}
                <div className="flex items-center gap-4">
                  <button
                    onClick={() => handleLike(comment.id)}
                    className={`flex items-center gap-1 text-xs transition-colors ${
                      comment.isLiked
                        ? 'text-red-400'
                        : 'text-gray-400 hover:text-red-400'
                    }`}
                  >
                    <Heart className={`w-3 h-3 ${comment.isLiked ? 'fill-current' : ''}`} />
                    {comment.likes}
                  </button>
                  
                  <button className="flex items-center gap-1 text-xs text-gray-400 hover:text-white transition-colors">
                    <Reply className="w-3 h-3" />
                    Reply
                  </button>
                  
                  <button className="opacity-0 group-hover:opacity-100 text-gray-400 hover:text-white transition-all">
                    <MoreVertical className="w-3 h-3" />
                  </button>
                </div>
                
                {/* Replies */}
                {comment.replies && comment.replies.length > 0 && (
                  <div className="ml-4 mt-3 space-y-3 border-l-2 border-gray-700 pl-4">
                    {comment.replies.map((reply) => (
                      <div key={reply.id} className="flex gap-2">
                        <div className="w-6 h-6 bg-gray-700 rounded-full flex items-center justify-center flex-shrink-0">
                          <User className="w-3 h-3 text-gray-400" />
                        </div>
                        <div className="flex-1">
                          <div className="flex items-center gap-2 mb-1">
                            <span className="font-medium text-white text-xs">{reply.author}</span>
                            {reply.position && (
                              <span className={`px-1.5 py-0.5 rounded-full text-xs font-medium ${
                                reply.position === 'YES' 
                                  ? 'bg-green-500/20 text-green-400' 
                                  : 'bg-red-500/20 text-red-400'
                              }`}>
                                {reply.position}
                              </span>
                            )}
                            <span className="text-gray-500 text-xs">{formatTimeAgo(reply.timestamp)}</span>
                          </div>
                          <p className="text-gray-200 text-xs leading-relaxed">{reply.content}</p>
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </div>
          </div>
        ))}
      </div>

      {/* Load More */}
      <div className="text-center mt-6">
        <Button
          variant="outline"
          size="sm"
          className="border-gray-600 text-gray-300 hover:text-white hover:border-gray-500"
        >
          Load more comments
        </Button>
      </div>
    </div>
  );
}