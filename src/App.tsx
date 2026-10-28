import { LibraryView } from "./views/LibraryView";
import { StatusBar } from "./components/StatusBar";

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
