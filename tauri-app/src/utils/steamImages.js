const STEAM_CDN = 'https://cdn.akamai.steamstatic.com/steam/apps';

export const getGameImage = (appId, type) => {
  if (!appId) return null;

  const types = {
    header: `${STEAM_CDN}/${appId}/header.jpg`,
    capsule: `${STEAM_CDN}/${appId}/capsule_616x353.jpg`,
    hero: `${STEAM_CDN}/${appId}/library_hero.jpg`,
    screenshot: (num = 1) => `${STEAM_CDN}/${appId}/ss_${num}.jpg`,
    background: `${STEAM_CDN}/${appId}/page_bg_generated.jpg`,
    icon: `${STEAM_CDN}/${appId}/capsule_184x69.jpg`,
  };

  if (typeof types[type] === 'function') {
    return types[type]();
  }

  return types[type] || null;
};

export default {
  getGameImage,
}; 