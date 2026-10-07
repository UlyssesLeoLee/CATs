// 自写 hash router（Svelte 5 runes）
//
// 提供一个全局响应式路由状态, 各页面通过 $derived 拿到当前 route。

export const route = $state({
  current: { name: "login", params: new URLSearchParams() } as {
    name: string;
    params: URLSearchParams;
  },
});

export function navigate(name: string) {
  location.hash = `#/${name}`;
  // hashchange 监听会更新 route.current
}