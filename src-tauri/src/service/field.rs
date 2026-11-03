use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use formats::field::{FieldMap, Layout};
use media::ContentDto;

use crate::model::{FieldMapDto, FieldOverviewDto, FieldPropDto};

const RENDER_CAPACITY: usize = 6;

struct LayoutEntry {
    path: String,
    size: u64,
    mtime_ms: u64,
    layout: Arc<Layout>,
    maps: Arc<Vec<FieldMap>>,
}

struct Rendered {
    key: String,
    dto: Arc<ContentDto>,
}

#[derive(Clone)]
pub struct FieldCache {
    layout: Arc<Mutex<Option<LayoutEntry>>>,
    renders: Arc<Mutex<VecDeque<Rendered>>>,
    overviews: Arc<Mutex<VecDeque<(String, Arc<FieldOverviewDto>)>>>,
}

impl Default for FieldCache {
    fn default() -> Self {
        Self::new()
    }
}

impl FieldCache {
    pub fn new() -> Self {
        Self {
            layout: Arc::new(Mutex::new(None)),
            renders: Arc::new(Mutex::new(VecDeque::new())),
            overviews: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    pub fn open(
        &self,
        path: &str,
        size: u64,
        mtime_ms: u64,
        bytes: &[u8],
    ) -> Result<(Arc<Layout>, Arc<Vec<FieldMap>>), String> {
        {
            let guard = self.layout.lock().map_err(|error| error.to_string())?;
            if let Some(entry) = guard.as_ref()
                && entry.path == path
                && entry.size == size
                && entry.mtime_ms == mtime_ms
            {
                return Ok((entry.layout.clone(), entry.maps.clone()));
            }
        }

        let layout = formats::field::parse(bytes).ok_or("the field layout did not parse")?;
        let maps = Arc::new(formats::field::maps(&layout));
        let layout = Arc::new(layout);

        let mut guard = self.layout.lock().map_err(|error| error.to_string())?;
        *guard = Some(LayoutEntry {
            path: path.to_string(),
            size,
            mtime_ms,
            layout: layout.clone(),
            maps: maps.clone(),
        });
        Ok((layout, maps))
    }

    pub fn maps(&self) -> Option<Arc<Vec<FieldMap>>> {
        self.layout
            .lock()
            .ok()?
            .as_ref()
            .map(|entry| entry.maps.clone())
    }

    pub fn cached(&self, key: &str) -> Option<Arc<ContentDto>> {
        let guard = self.renders.lock().ok()?;
        guard
            .iter()
            .find(|entry| entry.key == key)
            .map(|entry| entry.dto.clone())
    }

    pub fn overview(&self, key: &str) -> Option<Arc<FieldOverviewDto>> {
        let guard = self.overviews.lock().ok()?;
        guard
            .iter()
            .find(|(entry, _)| entry == key)
            .map(|(_, dto)| dto.clone())
    }

    pub fn store_overview(&self, key: String, dto: Arc<FieldOverviewDto>) {
        let Ok(mut guard) = self.overviews.lock() else {
            return;
        };
        guard.retain(|(entry, _)| *entry != key);
        guard.push_front((key, dto));
        while guard.len() > 4 {
            guard.pop_back();
        }
    }

    pub fn store(&self, key: String, dto: Arc<ContentDto>) {
        let Ok(mut guard) = self.renders.lock() else {
            return;
        };
        guard.retain(|entry| entry.key != key);
        guard.push_front(Rendered { key, dto });
        while guard.len() > RENDER_CAPACITY {
            guard.pop_back();
        }
    }
}

pub fn to_dto(entry: &FieldMap) -> FieldMapDto {
    FieldMapDto {
        name: entry.name.clone(),
        group: entry.group.clone(),
        files: entry.files.clone(),
        layers: entry.layers as u32,
        models: entry.models as u32,
        props: entry
            .props
            .iter()
            .map(|prop| FieldPropDto {
                name: prop.name.clone(),
                count: prop.count,
            })
            .collect(),
        tiles: entry.tiles as u32,
        window: entry.window,
    }
}
