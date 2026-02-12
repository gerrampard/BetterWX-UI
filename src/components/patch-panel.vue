<template>
    <CoexistItem v-if="rule?.hfeatures?.length" title="功能区" :inhead="true" :features="rule.hfeatures"
        @event="handleEvent"></CoexistItem>
    <Divider class="!m-0" />
    <ScrollPanel class="flex-1 min-h-0">
        <template v-for="(rule, index) in files" :key="index">
            <CoexistItem :title="rule.name" :features="rule.features" @event="handleEvent" :num="rule.index"
                :note="notes[rule.index]">
            </CoexistItem>
            <Divider class="!m-0" />
        </template>
    </ScrollPanel>
    <Divider class="!m-0" />
    <div class="flex flex-row items-center justify-between h-16 border-t-1 my-border-color">
        <div class="flex-1">
            <Tag :value="installTag.name" :severity="rule.installed ? 'success' : 'danger'" class="mr-1"></Tag>
            <Tag :value="patternTag.name" :severity="rule.supported ? 'success' : 'danger'"></Tag>
        </div>
        <div class="flex-1 h-16 flex flex-row justify-end items-center">
            <span v-if="rule.news" class="!whitespace-pre-line text-sm">{{ rule.news }}</span>
        </div>
    </div>
    <Dialog v-model:visible="inputDialog.show" modal :header="inputDialog.title" class="w-120" :closable="false">
        <div v-if="inputDialog.detaildesc" class="text-ellipsis mb-2 font-bold">{{ inputDialog.detaildesc }}</div>
        <div class="flex flex-col">
            <template v-for="(item, index) in inputDialog.ovs" :key="index">
                <div class="flex flex-col justify-center">
                    <div class="text-ellipsis mb-1">{{ item.pname }}</div>
                    <InputText class="w-full mb-1" type="text" v-model="item.text" :invalid="inputInvalid(item)">
                    </InputText>
                </div>
            </template>
            <div class="flex justify-end mt-3">
                <Button class="mr-2" type="button" label="取消" severity="secondary" @click="input_cancle"
                    size="small"></Button>
                <Button type="button" label="确认" @click="input_confirm" size="small"></Button>
            </div>
        </div>
    </Dialog>
    <Loading :show="showLoading" />
</template>

<script setup>
import { ref, watch, inject, computed, nextTick } from "vue"
import CoexistItem from "@/components/coexist-item.vue"
import * as ruleApis from "@/apis/rule.js"
import * as cmdApis from "@/apis/cmd.js"
import * as shortcutApis from "@/apis/shortcut.js"
import * as processApis from "@/apis/process.js"
import * as storeApis from "@/apis/store.js"
import * as tools from "@/utils/tools.js"
import { sleep } from "@/utils/tools"

const props = defineProps({
    data: { type: Object, default: {} },
    init: { type: Boolean, default: false }
})

const select_store_key = ref("")
const note_store_key = ref("")
const notes = ref({})
const showToast = inject('showToast')
const showToastInfo = inject('showToastInfo')

const inited = ref(false)
const initError = ref("")
const rule = ref({})
const files = ref([])
const nums = ref(new Set())
const showLoading = ref(false)
const inputDialog = ref({})

watch(() => props.init, async (newValue) => {
    if (newValue) {
        if (initError.value) {
            showToast(initError.value)
            return
        }
        if (!inited.value) {
            select_store_key.value = `${props.data.code}_select`
            note_store_key.value = `${props.data.code}_note`
            init()
            inited.value = true
        }
    }

}, { immediate: true })

async function init() {
    await nextTick()
    showLoading.value = true
    try {
        rule.value = await ruleApis.rule_get_path(props.data.code);
        console.log("获取安装路径", rule.value);
        if (!rule.value.installed) {
            console.error(installTag.value.name);
            return
        }
        let p1 = await ruleApis.rule_search_address(props.data.code);
        console.log("获取搜索基址", p1);
        rule.value = { ...rule.value, ...p1 }
        files.value = await ruleApis.rule_walk_files(props.data.code);
        console.log("获取文件列表", files.value);
        nums.value = new Set(files.value.map(file => file.index))
        set_select()
        set_note()
    } catch (error) {
        initError.value = error
        showToast(initError.value)
        console.error(error);
    } finally {
        showLoading.value = false
    }
}

async function handleEvent(data) {
    if (showLoading.value) {
        showToast("请等待上一个操作完成")
        return
    }
    showLoading.value = true
    try {
        await handleMethod(data)
        if (data.status && data.feature.tips) {
            showToastInfo(data.feature.tips)
        }
    } catch (error) {
        showToast(error)
        console.error(error);
    } finally {
        showLoading.value = false
    }
}

async function handleMethod(data) {
    let method = data.feature.method || data.feature.code
    console.log(method, data);
    switch (method) {
        case "folder":
            await cmdApis.cmd_open_folder(data.feature.target)
            break;
        case "open":
            await processApis.process_run_app(data.feature.target)
            break;
        case "close":
            await close(data.feature)
            break;
        case "lnk":
            await lnk(data)
            break;
        case "lnk_all":
            await lnk_all(data)
            break;
        case "coexist":
            await coexist(data)
            break;
        case "del":
            await del(data)
            break;
        case "patch":
            await patch(data)
            break;
        case "select_all":
            await select_all(data)
            break;
        case "select":
            await select(data)
            break;
        case "note":
            await note(data)
            break;
        case "open_all":
            await open_all(data)
            break;
        case "close_all":
            await close_all(data)
            break;
        case "patch_input":
            await patch_input(data)
            break;
        default:
            throw new Error(`未定义的方法：${method}`)
    }
}

async function close(feature, delay = 0) {
    let closed = await processApis.process_close_app(feature.target)
    if (closed && delay) {
        await sleep(delay)
    }
}

async function lnk(data) {
    let text = notes.value[data.num] || data.num
    let name = `${props.data.name}${text ? '#' + text : ""}`
    await shortcutApis.shortcut_to_desktop(data.feature.target, name)
}

async function lnk_all(data) {
    let features = get_same_code_feature("select")?.filter(feature => feature.selected)
    if (features.length == 0) {
        throw new Error("请先选择要启动的程序")
    }
    let list = features.map(feature => feature.target).join(",")
    let name = `${props.data.name}#一键启动`
    let icon = data.feature.target
    let path = rule.value.hfeatures.find(feature => feature.code == "folder").target
    let args = {
        code: props.data.code,
        name: props.data.name,
        path: path,
        list: list,
        login: rule.value.hfeatures.find(feature => feature.code == "open_all").target,
    }
    await shortcutApis.shortcut_to_desktop(null, name, icon, args)
    console.log(list);
}

async function coexist() {
    let num = 0
    for (let i = 1; i <= 9; i++) {
        if (!nums.value.has(i)) {
            num = i
            break
        }
    }
    if (!num) {
        throw new Error("不能创建更多的共存的了")
    }
    nums.value.add(num)
    try {
        let file = await ruleApis.rule_make_coexist(props.data.code, num)
        files.value.splice(num, 0, file)
        select()
    } catch (error) {
        nums.value.delete(num)
        throw error
    }
}

async function del(data) {
    let index = files.value.findIndex(file => file.index == data.num)
    let file = files.value[index]
    let cfeature = file?.features.find(feature => feature.code == "close")
    await close(cfeature, 1000)
    await ruleApis.rule_del_coexist(props.data.code, data.num)
    files.value.splice(index, 1)
    nums.value.delete(data.num)
    select()
}

async function patch(data) {
    data.feature.status = data.status
    let file = get_file_by_num(data.num)
    let cfeature = file?.features.find(feature => feature.code == "close")
    try {
        await close(cfeature, 1000)
        let views = await ruleApis.rule_patch(props.data.code, data.num, data.feature.code, data.status)
        console.log("修补后开启的功能", views);
        file.features.forEach(feature => {
            if (feature.method == "patch") {
                feature.status = views.includes(feature.code)
            }
        })
    } catch (error) {
        data.feature.status = !data.status
        throw error
    }

}

async function select_all(data) {
    let features = get_same_code_feature("select")
    features.forEach(feature => feature.selected = data.status)
    select()
}

async function select() {
    if (!select_store_key.value) {
        showToast("缓存文件名无效")
        return
    }
    let nfiles = files.value.filter(file => file.features.find(feature => feature.code == "select" && feature.selected))
    let ns = nfiles.map(file => file.index)?.join(",")
    storeApis.store_save(select_store_key.value, ns)
}

async function set_select() {
    if (!select_store_key.value) {
        showToast("缓存文件名无效")
        return
    }
    let select = await storeApis.store_read(select_store_key.value)
    let ns = select.split(",")
    files.value.forEach(file => {
        file.features.forEach(feature => {
            if (feature.code == "select") {
                feature.selected = ns?.includes(file.index.toString())
            }
        })
    })
}

async function set_note() {
    if (!note_store_key.value) {
        showToast("缓存文件名无效")
        return
    }
    try {
        let notes_str = await storeApis.store_read(note_store_key.value)
        if (!notes_str) {
            return
        }
        notes.value = JSON.parse(notes_str)
        return notes.value
    } catch (_) {
        console.log(`${note_store_key.value} 未配置备注`);
    }
}

async function note(data) {
    let note_text = notes.value[data.num]
    let ovs = [
        {
            pname: data.feature.name,
            text: note_text,
            len: 40
        }
    ]
    setDialogData(data,"note",ovs)
}

async function open_all(data) {
    let features = get_same_code_feature("select")
    let ffeatures = features.filter(feature => feature.selected)
    if (ffeatures.length == 0) {
        throw new Error("请先选择要启动的程序")
    }
    let exes = ffeatures.map(feature => `${rule.value.install_location}\\${feature.target}`)
    console.log(exes, data.feature.target);
    await processApis.process_run_apps(exes, data.feature.target)
}

async function close_all(data) {
    let features = get_same_code_feature("select")
    let ffeatures = features.filter(feature => feature.selected)
    if (ffeatures.length == 0) {
        throw new Error("请先选择要关闭的程序")
    }
    let exes = ffeatures.map(feature => feature.target)
    await processApis.process_close_apps(exes)
}

async function patch_input(data) {
    let ovs = await ruleApis.rule_read_orignal(props.data.code, data.num, data.feature.code)
    ovs.forEach(item => {
        item.text = tools.hex2text(item.orignal)
    })
    setDialogData(data,"patch",ovs)
}

function setDialogData(data,type,ovs) {
    inputDialog.value.fcode = data.feature.code
    inputDialog.value.num = data.num
    inputDialog.value.show = true
    inputDialog.value.title = data.feature.description || data.feature.name
    inputDialog.value.description = data.feature.description
    inputDialog.value.detaildesc = data.feature.detaildesc
    inputDialog.value.type = type
    inputDialog.value.ovs = ovs
}

async function input_cancle() {
    inputDialog.value.show = false
}

async function input_confirm() {
    switch (inputDialog.value.type) {
        case "patch":
            await input_confirm_patch()
            break;
        case "note":
            await input_confirm_note()
            break;
        default:
            throw new Error(`为实现的方法：${inputDialog.value.type}`)
    }
    inputDialog.value.show = false
}

async function input_confirm_note() {
    let item = inputDialog.value.ovs[0]
    if (item.text.length / 2 > item.len) {
        showToast(`${item.pname} 输入的内容太多了`)
        return
    }
    let num = inputDialog.value.num
    let note = item.text
    notes.value[num] = note
    await storeApis.store_save(note_store_key.value, JSON.stringify(notes.value))

}

async function input_confirm_patch() {
    for (let item of inputDialog.value.ovs) {
        if (item.text.length == 0) {
            showToast(`${item.pname} 输入的内容不能为空`)
            return
        }
        let text = tools.text2hex(item.text)
        if (text.length / 2 > item.len) {
            showToast(`${item.pname} 输入的内容太多了`)
            return
        }
        item.orignal = text
    }
    let num = inputDialog.value.num
    let fcode = inputDialog.value.fcode
    let ovs = inputDialog.value.ovs
    let file = get_file_by_num(num)
    let cfeature = file?.features.find(feature => feature.code == "close")
    await close(cfeature, 1000)
    await ruleApis.rule_patch_by_replace(props.data.code, num, fcode, ovs)
}

function get_file_by_num(num) {
    let file = files.value.find(file => file.index == num)
    if (!file) {
        throw new Error("不存在该共存，请重启软件")
    }
    return file
}

function get_same_code_feature(code) {
    return files.value.map(file => {
        return file.features.find(feature => feature.code == code)
    })
}

const installTag = computed(() => {
    if (rule.value.installed) {
        return {
            name: `安装的版本： ${rule.value.install_version}`,
            severity: "success"
        }
    }
    return {
        name: `未检测到： ${props.data.name}`,
        severity: "danger"
    }
})

const patternTag = computed(() => {
    let severity
    if (rule.value.supported) {
        severity = "success"
    } else {
        severity = "danger"
    }
    return {
        name: `特征码版本： ${props.data.version}`,
        severity
    }
})

const inputInvalid = computed(() => (data) => {
    let text = tools.text2hex(data.text)
    if (text.length / 2 > data.len) {
        return true
    }
    return false
})

</script>