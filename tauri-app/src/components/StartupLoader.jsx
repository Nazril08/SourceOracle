import React from 'react';
import { IoGameController } from 'react-icons/io5';

const SkeletonCard = () => (
  <div className="bg-card rounded-lg overflow-hidden shadow-lg animate-pulse">
    <div className="w-full h-32 bg-sidebar"></div>
    <div className="p-4">
      <div className="h-4 bg-sidebar rounded w-3/4 mb-2"></div>
      <div className="h-3 bg-sidebar rounded w-1/2"></div>
    </div>
  </div>
);

const StartupLoader = () => {
  return (
    <div className="flex h-screen bg-background text-white overflow-hidden">
      {/* Sidebar Skeleton */}
      <div className="w-48 bg-sidebar flex flex-col justify-between border-r border-border animate-pulse">
        <div>
          <div className="p-6">
            <div className="h-8 bg-surface rounded w-3/4"></div>
          </div>
          <nav className="mt-6 space-y-3 px-4">
            <div className="h-8 bg-surface rounded"></div>
            <div className="h-8 bg-surface rounded"></div>
            <div className="h-8 bg-surface rounded"></div>
            <div className="h-8 bg-surface rounded"></div>
          </nav>
        </div>
        <div className="p-4 border-t border-border">
          <div className="h-10 bg-surface rounded"></div>
        </div>
      </div>

      {/* Main Content Skeleton */}
      <main className="flex-1 flex flex-col overflow-hidden p-6">
        <div className="flex justify-between items-center mb-6">
          <div className="h-8 bg-sidebar rounded w-1/4"></div>
          <div className="h-10 bg-sidebar rounded-full w-64"></div>
        </div>
        <div className="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-4">
          {Array.from({ length: 10 }).map((_, i) => (
            <SkeletonCard key={i} />
          ))}
        </div>
        <div className="text-center mt-8 text-gray-400 text-lg flex items-center justify-center gap-3">
            <IoGameController className="text-primary text-3xl animate-bounce"/>
            Initializing Application...
        </div>
      </main>
    </div>
  );
};

export default StartupLoader; 