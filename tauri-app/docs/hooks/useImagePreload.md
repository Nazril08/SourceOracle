# useImagePreload Hook

Custom hook untuk pre-loading gambar dengan handling loading state dan error.

## Usage

```jsx
import useImagePreload from '../hooks/useImagePreload';

// Basic usage
const { isLoaded, error } = useImagePreload(imageUrl);

// Dengan Steam AppID
const { isLoaded, error } = useImagePreload(`https://cdn.akamai.steamstatic.com/steam/apps/${appId}/header.jpg`);
```

## Parameters

- `imageUrl` (string): URL gambar yang akan di-preload

## Returns

Object dengan properties:
- `isLoaded` (boolean): Status loading gambar
- `error` (string|null): Pesan error jika gagal load, null jika sukses

## Examples

### 1. Basic Image Loading dengan Loading State

```jsx
const MyComponent = ({ imageUrl }) => {
  const { isLoaded, error } = useImagePreload(imageUrl);
  
  return (
    <div className="relative">
      {/* Loading Skeleton */}
      {!isLoaded && (
        <div className="absolute inset-0 bg-sidebar animate-pulse">
          <LoadingSpinner />
        </div>
      )}
      
      {/* Image */}
      <img 
        src={imageUrl}
        className={`transition-opacity duration-300 ${
          isLoaded ? 'opacity-100' : 'opacity-0'
        }`}
      />
      
      {/* Error State */}
      {error && <ErrorMessage message={error} />}
    </div>
  );
};
```

### 2. Steam Game Header Image

```jsx
const GameHeader = ({ appId }) => {
  const headerUrl = `https://cdn.akamai.steamstatic.com/steam/apps/${appId}/header.jpg`;
  const { isLoaded, error } = useImagePreload(headerUrl);
  
  return (
    <div className="relative w-96 h-48 bg-sidebar rounded-lg overflow-hidden">
      {/* Loading State */}
      {!isLoaded && (
        <div className="absolute inset-0 flex items-center justify-center">
          <IoGameController className="text-primary text-6xl animate-bounce" />
        </div>
      )}
      
      {/* Game Image */}
      {!error ? (
        <img 
          src={headerUrl}
          alt={`Game ${appId} header`}
          className={`w-full h-full object-cover transition-opacity duration-300 ${
            isLoaded ? 'opacity-100' : 'opacity-0'
          }`}
        />
      ) : (
        <div className="w-full h-full flex items-center justify-center">
          <IoGameController className="text-primary text-6xl" />
        </div>
      )}
    </div>
  );
};
```

### 3. Multiple Image Types

```jsx
const GameImages = ({ appId }) => {
  // Header image
  const headerUrl = `https://cdn.akamai.steamstatic.com/steam/apps/${appId}/header.jpg`;
  const header = useImagePreload(headerUrl);
  
  // Capsule image
  const capsuleUrl = `https://cdn.akamai.steamstatic.com/steam/apps/${appId}/capsule_616x353.jpg`;
  const capsule = useImagePreload(capsuleUrl);
  
  // Library hero image
  const heroUrl = `https://cdn.akamai.steamstatic.com/steam/apps/${appId}/library_hero.jpg`;
  const hero = useImagePreload(heroUrl);
  
  return (
    <div className="grid gap-4">
      {/* Render images with their respective loading states */}
      <ImageWithLoading url={headerUrl} state={header} type="header" />
      <ImageWithLoading url={capsuleUrl} state={capsule} type="capsule" />
      <ImageWithLoading url={heroUrl} state={hero} type="hero" />
    </div>
  );
};
```

## Steam Image URL Patterns

Common Steam CDN URL patterns untuk gambar game:

```javascript
const STEAM_CDN = 'https://cdn.akamai.steamstatic.com/steam/apps';

const getGameImage = (appId, type) => {
  const types = {
    header: `${STEAM_CDN}/${appId}/header.jpg`,
    capsule: `${STEAM_CDN}/${appId}/capsule_616x353.jpg`,
    hero: `${STEAM_CDN}/${appId}/library_hero.jpg`,
    screenshot: (num) => `${STEAM_CDN}/${appId}/ss_${num}.jpg`,
    background: `${STEAM_CDN}/${appId}/page_bg_generated.jpg`
  };
  
  return types[type];
};
```

## Tips

1. Selalu sediakan fallback UI untuk error state
2. Gunakan loading skeleton untuk UX yang lebih baik
3. Implementasikan transisi smooth saat gambar loaded
4. Cache gambar yang sudah di-load untuk penggunaan berikutnya
5. Perhatikan memory management dengan cleanup di useEffect 