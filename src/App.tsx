import { LibraryView } from "./views/LibraryView";
import { StatusBar } from "./components/StatusBar";

/** 单视图应用：库 = 目录树 + 文件列表/网格 + 查看器（ADR-0012 取代三视图路由）。 */
export default function App() {
  return (
    <div className="flex h-screen w-screen flex-col overflow-hidden bg-base-100 text-base-content">
      <main className="flex min-h-0 flex-1 flex-col overflow-hidden">
        <LibraryView />
      </main>
      <StatusBar />
    </div>
  );
}
