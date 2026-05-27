import { Navigate, useLocation } from 'react-router-dom';
import { LEGACY_PATH_REDIRECTS } from '@/data/navConfig';

/** 旧 URL → 新路由（保留 query） */
export function LegacyRedirect() {
  const location = useLocation();
  const target = LEGACY_PATH_REDIRECTS[location.pathname];

  if (target) {
    const [path, query] = target.split('?');
    const merged = query
      ? `${path}?${query}${location.search ? `&${location.search.slice(1)}` : ''}`
      : path + location.search;
    return <Navigate to={merged} replace />;
  }

  return <Navigate to="/factory" replace />;
}
