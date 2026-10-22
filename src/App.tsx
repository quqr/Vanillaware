import { LibraryView } from "./views/LibraryView";
import { StatusBar } from "./components/StatusBar";

/** 单视图应用：库 = 目录树 + 文件列表/网格 + 查看器（ADR-0012 取代三视图路由）。 */
export default function App() {
  return (
    <div className="app">
      <main className="view">
        <LibraryView />
      </main>
      <StatusBar />
    </div>
  );
}
