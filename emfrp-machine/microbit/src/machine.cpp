#ifndef INCLUDE_MACHINE
#define INCLUDE_MACHINE
#include "machine.h"
#endif
#include <string.h>
#define READ_U8() (*ip++)
#define READ_I8() (int8_t)(*ip++)
#define READ_I32(dest)                                                               \
    dest = (int)ip[0] + ((int)ip[1] << 8) + ((int)ip[2] << 16) + ((int)ip[3] << 24); \
    ip += 4;
#define READ_I16(dest)                     \
    dest = (int)ip[0] + ((int)ip[1] << 8); \
    ip += 2;
#define READ_U32(dest)                                                                                   \
    dest = (uint32_t)ip[0] + ((uint32_t)ip[1] << 8) + ((uint32_t)ip[2] << 16) + ((uint32_t)ip[3] << 24); \
    ip += 4;
#define PUSH_PTR(val) sp++->ptr = (val)
#define PUSH_NUM(val) sp++->num = (val)
#define PUSH(val) *sp++ = (val)
#define POP_NUM() ((--sp)->num)
#define POP_PTR() ((--sp)->ptr)
#define POP() (*--sp)
// header tag:7bit/ reserved:1bit/ numentry:3bit/ objbit:7bit/ refcnt:14bit
#define OBJ_HEADER(vobj) ((vobj).obj->obj_header)
#define OBJ_TAG(vobj) (OBJ_HEADER(vobj) >> 25)
#define OBJ_ENTRY_NUM(vobj) ((OBJ_HEADER(vobj) & 0xe00000) >> 21)
#define OBJBIT_ITH(vobj, i) ((OBJ_HEADER(vobj) & (1 << (i + 14))) != 0)
#define OBJ_ITH(vobj, i) ((vobj).obj[(i) + 1])
#define OBJ_INC_RC(vobj) OBJ_HEADER(vobj) += 1
#define OBJ_DEC_RC_SHALLOW(vobj) OBJ_HEADER(vobj) -= 1
#define OBJ_GET_RC(vobj) (OBJ_HEADER(vobj) & 0x3fff)
#define INITIAL_VARIABLE_CAPACITY 3
#define STACK_SIZE 128
#ifdef EMFRP_MEASURE_HEAP
static uint32_t min_heap_free_size;
#endif
#ifdef EMFRP_DEBUG
static int obj_cnt = 0;
#endif

static value_t ZERO;
static value_t OBJ_INIT_VALUE;

#ifdef EMFRP_MEASURE_HEAP
uint32_t get_min_free_heap_size()
{
    return min_heap_free_size;
}
static void update_min_free_heap_size(char *dbg_info)
{
    uint32_t v = esp_get_free_heap_size();
#ifdef EMFRP_MEASURE_HEAP
    ESP_LOGI("[HEAP]", "%s: %lu", dbg_info, v);
#endif
    if (v < min_heap_free_size)
    {

        min_heap_free_size = v;
    }
}

void init_min_heap_free_size(uint32_t v)
{
    min_heap_free_size = v;
}

#endif

static void drop(value_t obj)
{
    OBJ_DEC_RC_SHALLOW(obj);
    if (!OBJ_GET_RC(obj))
    {
        for (int i = 0, j = OBJ_ENTRY_NUM(obj); i < j; ++i)
        {
            if (OBJBIT_ITH(obj, i))
            {
                drop(OBJ_ITH(obj, i));
            }
        }
#ifdef EMFRP_DEBUG
        --obj_cnt;
#endif
        free(obj.obj);
    }
}

static inline value_t *alloc_obj(emfrp_machine *em, int len)
{
#ifdef EMFRP_DEBUG
    ++obj_cnt;
#endif
#ifdef EMFRP_MEASURE_HEAP

    value_t *v = (value_t *)malloc(sizeof(value_t) * len);
    update_min_free_heap_size("alloc obj");
    return v;

#else
    return (value_t *)malloc(sizeof(value_t) * len);
#endif
}

static inline int next_int(uint8_t **p)
{ // little endian
    int ret = (int)(**p) + (((int)(p[0][1])) << 8) + (((int)(p[0][2])) << 16) + (((int)(p[0][3])) << 24);
    *p += 4;
    return ret;
}
static inline uint8_t next_byte(uint8_t **p)
{

    uint8_t ret = **p;
    *p += 1;
    return ret;
}
void init_node_list(node_list_t *nd_list, int cap)
{
    nd_list->len = 0;
    nd_list->cap = cap;
    nd_list->values = (value_t *)malloc(sizeof(value_t) * cap);
    nd_list->action = (upd_action_t *)malloc(sizeof(upd_action_t) * cap);
}
void init_func_list(func_list_t *f_list, int cap)
{
    f_list->len = 0;
    f_list->cap = cap;
    f_list->lst = (func_t *)malloc(sizeof(func_t) * cap);
}
void init_data_list(data_list_t *d_list, int cap)
{
    d_list->len = 0;
    d_list->cap = cap;
    d_list->lst = (data_t *)malloc(sizeof(data_t) * cap);
}

static inline void push_node_usr_uncheck(node_list_t *lst, value_t v, uint8_t *upd)
{
    int len = lst->len;
    lst->values[len] = v;
    lst->action[len].insns = upd;
    lst->len += 1;
}
static inline void push_node_dev_uncheck(node_list_t *lst, value_t v, dev_input_t driver)
{
    int len = lst->len;
    lst->values[len] = v;
    lst->action[len].dev = driver;
    lst->len += 1;
}

void extend_node_list(node_list_t *lst, const int new_cap)
{
    if (lst->cap < new_cap)
    {
        value_t *vs = (value_t *)malloc(sizeof(value_t) * new_cap);
        memcpy(vs, lst->values, lst->len * sizeof(value_t));
#ifdef EMFRP_MEASURE_HEAP
        update_min_free_heap_size("extend node list");
#endif
        free(lst->values);
        upd_action_t *upds = (upd_action_t *)malloc(sizeof(upd_action_t) * new_cap);
        memcpy(upds, lst->action, lst->len * sizeof(upd_action_t));

#ifdef EMFRP_MEASURE_HEAP
        update_min_free_heap_size("extend node list");
#endif
        free(lst->action);

        lst->action = upds;
        lst->values = vs;
        lst->cap = new_cap;
    }
}
void extend_func_list(func_list_t *lst, const int new_cap)
{

    if (lst->cap < new_cap)
    {
        func_t *f = (func_t *)malloc(sizeof(func_t) * new_cap);
        memcpy(f, lst->lst, lst->len * sizeof(func_t));

#ifdef EMFRP_MEASURE_HEAP
        update_min_free_heap_size("extend func list");
#endif
        free(lst->lst);
        lst->lst = f;
        lst->cap = new_cap;
    }
}
void extend_data_list(data_list_t *lst, const int new_cap)
{
    if (lst->cap < new_cap)
    {
        data_t *d = (data_t *)malloc(sizeof(data_t) * new_cap);
        memcpy(d, lst->lst, lst->len * sizeof(data_t));

#ifdef EMFRP_MEASURE_HEAP
        update_min_free_heap_size("extend data list");
#endif
        free(lst->lst);
        lst->lst = d;
        lst->cap = new_cap;
    }
}
static inline void push_func_uncheck(func_list_t *lst, func_t f)
{

    lst->lst[lst->len] = f;
    lst->len += 1;
}
static inline void push_data_uncheck(data_list_t *lst, data_t d)
{
    lst->lst[lst->len] = d;
    lst->len += 1;
}

void emfrp_init(emfrp_machine *em, int n_input_node, int n_output_node)
{
    ZERO.num = 0;
    em->v_stack = (value_t *)malloc(sizeof(value_t) * STACK_SIZE);

    init_node_list(&em->node_list, n_input_node + n_output_node);
    init_func_list(&em->func_list, INITIAL_VARIABLE_CAPACITY);
    init_data_list(&em->data_list, INITIAL_VARIABLE_CAPACITY);

    em->update = NULL;
    em->node_last = NULL;
    em->res = EMFRP_NOTSTARTED;
    em->output_actions = (output_action_t *)malloc(n_output_node * sizeof(output_action_t));
    em->output_nd_len = 0;
    value_t *obj = (value_t *)malloc(sizeof(value_t));
    obj->obj_header = 1;
    OBJ_INIT_VALUE.obj = obj;
}

value_t emfrp_int(int i)
{
    value_t v;
    v.num = i;
    return v;
}

void emfrp_add_input_node(emfrp_machine *em, value_t init, dev_input_t driver)
{
    push_node_dev_uncheck(&em->node_list, init, driver);
}
void emfrp_add_output_node(emfrp_machine *em, value_t init, output_action_t driver)
{
    em->output_actions[em->output_nd_len++] = driver;
    push_node_usr_uncheck(&em->node_list, init, NULL);
}

value_t emfrp_exec(emfrp_machine *em, uint8_t *ip)
{
    value_t *bp = em->v_stack;
    value_t *sp = em->v_stack;
    value_t *node_v = em->node_list.values;
    const upd_action_t *action = em->node_list.action;
    value_t *node_vlast = em->node_last;
    data_t *data = em->data_list.lst;
    const func_t *func = em->func_list.lst;
    const output_action_t *output_actions = em->output_actions;
    value_t tmp_v;
    uint8_t tmp_byte;
    int tmp_int;
    call_frame *frame_prev = NULL, *frame_tmp;

    while (1)
    {
#ifdef EMFRP_DEBUG
        printf("insn:%d, stack:", (int)*ip);
        for (int i = 0, j = sp - bp; i < j; ++i)
        {
            printf(" %d ", sp[i].num);
        }
        printf("\n");
#endif
        switch (*ip++)
        {
        case BC_None:
#ifdef EMFRP_DEBUG
            ESP_LOGI("[ERROR]", "invalid insn : BC_NONE\n");
#endif
            em->res = EMFRP_PANIC;
            return ZERO;
        case BC_Nil:
            PUSH(ZERO);
            break;
        case BC_Not:
            sp[-1].num = !sp[-1].num;
            break;
        case BC_Minus:
            sp[-1].num = -sp[-1].num;
            break;
        case BC_Add: // a b sp -> (a+b) sp
            tmp_int = POP_NUM();
            tmp_int += POP_NUM();
            PUSH_NUM(tmp_int);
            break;
        case BC_Sub: // a b sp -> (a+b) sp
            tmp_int = POP_NUM();
            tmp_int = POP_NUM() - tmp_int;
            PUSH_NUM(tmp_int);
            break;
        case BC_Mul: // a b sp -> (a+b) sp
            tmp_int = POP_NUM();
            tmp_int *= POP_NUM();
            PUSH_NUM(tmp_int);
            break;
        case BC_Div: // a b sp -> (a+b) sp
            tmp_int = POP_NUM();
            tmp_int = POP_NUM() / tmp_int;
            PUSH_NUM(tmp_int);
            break;
        case BC_Mod: // a b sp -> (a+b) sp
            tmp_int = POP_NUM();
            tmp_int = POP_NUM() % tmp_int;
            PUSH_NUM(tmp_int);
            break;
        case BC_Ls: // a b sp -> (a+b) sp
            tmp_int = POP_NUM();
            tmp_int = POP_NUM() < tmp_int;
            PUSH_NUM(tmp_int);
            break;
        case BC_Leq: // a b sp -> (a+b) sp
            tmp_int = POP_NUM();
            tmp_int = POP_NUM() <= tmp_int;
            PUSH_NUM(tmp_int);
            break;
        case BC_Gt: // a b sp -> (a+b) sp
            tmp_int = POP_NUM();
            tmp_int = POP_NUM() > tmp_int;
            PUSH_NUM(tmp_int);
            break;
        case BC_Geq: // a b sp -> (a+b) sp
            tmp_int = POP_NUM();
            tmp_int = POP_NUM() >= tmp_int;
            PUSH_NUM(tmp_int);
            break;
        case BC_Eq: // a b sp -> (a+b) sp
            tmp_int = POP_NUM();
            tmp_int = POP_NUM() == tmp_int;
            PUSH_NUM(tmp_int);
            break;
        case BC_Neq: // a b sp -> (a+b) sp
            tmp_int = POP_NUM();
            tmp_int = POP_NUM() != tmp_int;
            PUSH_NUM(tmp_int);
            break;
        case BC_INT_0:
            PUSH_NUM(0);
            break;
        case BC_INT_1:
            PUSH_NUM(1);
            break;
        case BC_INT_2:
            PUSH_NUM(2);
            break;
        case BC_INT_3:
            PUSH_NUM(3);
            break;
        case BC_INT_4:
            PUSH_NUM(4);
            break;
        case BC_INT_5:
            PUSH_NUM(5);
            break;
        case BC_INT_I8:
            PUSH_NUM((int)READ_I8());
            break;
        case BC_INT_I16:
            READ_I16(tmp_int);
            PUSH_NUM(tmp_int);
            break;
        case BC_INT_I32:
            READ_I32(tmp_int)
            PUSH_NUM(tmp_int);
            break;
        case BC_PushTrue:;
            PUSH_NUM(true);
            break;
        case BC_PushFalse:
            PUSH_NUM(false);
            break;
        case BC_J8:;
            tmp_int = (int)READ_I8();
            ip += tmp_int;
            break;
        case BC_J32:;
            READ_I32(tmp_int);
            ip += tmp_int;
            break;
        case BC_Je8:
            if (POP_NUM())
            {

                tmp_int = (int)READ_I8();
                ip += tmp_int;
            }
            else
            {
                ip += 1;
            }
            break;
        case BC_Jne8:
            if (!POP_NUM())
            {
                ;
                tmp_int = (int)READ_I8();
                ip += tmp_int;
            }
            else
            {
                ip += 1;
            }
            break;
        case BC_Jne32:
            if (!POP_NUM())
            {
                READ_I32(tmp_int);
                ip += tmp_int;
            }
            else
            {
                ip += 4;
            }
            break;
        case BC_GET_DATA_U8:
            PUSH(data[READ_U8()]);
            break;
        case BC_GET_DATA_U32:
            READ_U32(tmp_int);
            PUSH(data[tmp_int]);
            break;
        case BC_GET_LOCAL_0:
            PUSH(*bp);
            break;
        case BC_GET_LOCAL_1:
            PUSH(bp[1]);
            break;
        case BC_GET_LOCAL_2:
            PUSH(bp[2]);
            break;
        case BC_GET_LOCAL_3:
            PUSH(bp[3]);
            break;
        case BC_GET_LOCAL_4:
            PUSH(bp[4]);
            break;
        case BC_GET_LOCAL_5:
            PUSH(bp[5]);
            break;
        case BC_GET_LOCAL_6:
            PUSH(bp[6]);
            break;
        case BC_GET_LOCAL_I8:
            PUSH(*(bp + READ_I8()));
            break;
        case BC_GET_LOCAL_I32:
            READ_I32(tmp_int);
            PUSH(*(bp + tmp_int));
            break;
        case BC_SET_LOCAL_0:
            *bp = POP();
            break;
        case BC_SET_LOCAL_1:
            bp[1] = POP();
            break;
        case BC_SET_LOCAL_2:
            bp[2] = POP();
            break;
        case BC_SET_LOCAL_3:
            bp[3] = POP();
            break;
        case BC_SET_LOCAL_4:
            bp[4] = POP();
            break;
        case BC_SET_LOCAL_5:
            bp[5] = POP();
            break;
        case BC_SET_LOCAL_6:
            bp[6] = POP();
            break;
        case BC_SET_LOCAL_I8:
            bp[READ_I8()] = POP();
            break;
        case BC_SET_LOCAL_I32:
            READ_I32(tmp_int);
            bp[tmp_int] = POP();
            break;
        case BC_GET_NODE_U8:
            PUSH(node_v[READ_U8()]);
            break;
        case BC_GET_NODE_U32:
            READ_U32(tmp_int);
            PUSH(node_v[tmp_int]);
            break;
        case BC_GET_LAST_0:
            PUSH(node_vlast[0]);
            break;
        case BC_GET_LAST_1:
            PUSH(node_vlast[1]);
            break;
        case BC_GET_LAST_2:
            PUSH(node_vlast[2]);
            break;
        case BC_GET_LAST_3:
            PUSH(node_vlast[3]);
            break;
        case BC_GET_LAST_U8:
            PUSH(node_vlast[READ_U8()]);
            break;
        case BC_GET_LAST_U32:
            READ_U32(tmp_int);
            PUSH(node_vlast[tmp_int]);
            break;
        case BC_SET_LAST_0:
            node_vlast[0] = POP();
            break;
        case BC_SET_LAST_1:
            node_vlast[1] = POP();
            break;
        case BC_SET_LAST_2:
            node_vlast[2] = POP();
            break;
        case BC_SET_LAST_3:
            node_vlast[3] = POP();
            break;
        case BC_SET_LAST_U8:
            node_vlast[READ_U8()] = POP();
            break;
        case BC_SET_LAST_U32:
            READ_U32(tmp_int);
            node_vlast[tmp_int] = POP();
            break;
        case BC_CALL_U8:;
            tmp_byte = READ_U8(); // nargs
            frame_tmp = (call_frame *)malloc(sizeof(call_frame));
            frame_tmp->caller = frame_prev;
            frame_tmp->bp = bp;
            frame_tmp->sp = sp - tmp_byte;
            frame_prev = frame_tmp;
            bp = sp - tmp_byte;
            tmp_byte = READ_U8();
            frame_tmp->next_ip = ip;
            ip = func[tmp_byte];

            break;
        case BC_CALL_U32:
            tmp_byte = READ_U8(); // nargs
            frame_tmp = (call_frame *)malloc(sizeof(call_frame));
            frame_tmp->caller = frame_prev;
            frame_tmp->bp = bp;
            frame_tmp->sp = sp - tmp_byte;

            frame_prev = frame_tmp;
            bp = sp - tmp_byte;

            READ_U32(tmp_int);
            frame_tmp->next_ip = ip;
            ip = func[tmp_int];
            break;
        case BC_Return:
            tmp_v = POP();
            ip = frame_prev->next_ip;
            bp = frame_prev->bp;
            sp = frame_prev->sp;
            frame_tmp = frame_prev;
            frame_prev = frame_prev->caller;
            free(frame_tmp);
            PUSH(tmp_v);
            break;
        case BC_SET_DATA_U8:
            data[READ_U8()] = POP();
            break;
        case BC_SET_DATA_U32:
            READ_U32(tmp_int);
            data[tmp_int] = POP();
            break;
        case BC_SET_NODE_U8:
            node_v[READ_U8()] = POP();
            break;
        case BC_SET_NODE_U32:
            READ_U32(tmp_int);
            node_v[tmp_int] = POP();
            break;
        case BC_UPD_DEV_0:
            action[0].dev(node_v);
            break;
        case BC_UPD_DEV_1:
            action[1].dev(node_v + 1);
            break;
        case BC_UPD_DEV_2:
            action[2].dev(node_v + 2);
            break;
        case BC_UPD_DEV_3:
            action[3].dev(node_v + 3);
            break;
        case BC_UPD_DEV_U8:
            tmp_byte = READ_U8();
            action[tmp_byte].dev(node_v + tmp_byte);
            break;
        case BC_UPD_NODE_U8:
            tmp_byte = READ_U8();
            sp->ip = ip;
            ++sp;
            ip = action[tmp_byte].insns;
            break;
        case BC_UPD_NODE_U32:
            READ_U32(tmp_int);
            sp->ip = ip;
            ++sp;
            ip = action[tmp_int].insns;
            break;
        case BC_END_UPD_U8:
            node_v[READ_U8()] = POP();
            --sp;
            ip = sp->ip;
            break;
        case BC_END_UPD_U32:
            READ_U32(tmp_int);
            node_v[tmp_int] = POP();
            --sp;
            ip = sp->ip;
            break;
        case BC_ALLOC_OBJ_0:
            tmp_v.obj = alloc_obj(em, 1);
            READ_U32(tmp_int);
            tmp_v.obj->obj_header = (uint32_t)tmp_int;
            tmp_byte = OBJ_ENTRY_NUM(tmp_v);
            for (int i = 0; i < tmp_byte; ++i)
            {
                tmp_v.obj[tmp_byte - i] = POP();
            }
            PUSH(tmp_v);
            break;
        case BC_ALLOC_OBJ_1:
            tmp_v.obj = alloc_obj(em, 2);
            READ_U32(tmp_int);
            tmp_v.obj->obj_header = (uint32_t)tmp_int;
            tmp_byte = OBJ_ENTRY_NUM(tmp_v);
            for (int i = 0; i < tmp_byte; ++i)
            {
                tmp_v.obj[tmp_byte - i] = POP();
            }
            PUSH(tmp_v);
            break;
        case BC_ALLOC_OBJ_2:
            tmp_v.obj = alloc_obj(em, 3);
            READ_U32(tmp_int);
            tmp_v.obj->obj_header = (uint32_t)tmp_int;
            tmp_byte = OBJ_ENTRY_NUM(tmp_v);
            for (int i = 0; i < tmp_byte; ++i)
            {
                tmp_v.obj[tmp_byte - i] = POP();
            }
            PUSH(tmp_v);
            break;
        case BC_ALLOC_OBJ_3:
            tmp_v.obj = alloc_obj(em, 4);
            READ_U32(tmp_int);
            tmp_v.obj->obj_header = (uint32_t)tmp_int;
            tmp_byte = OBJ_ENTRY_NUM(tmp_v);
            for (int i = 0; i < tmp_byte; ++i)
            {
                tmp_v.obj[tmp_byte - i] = POP();
            }
            PUSH(tmp_v);
            break;
        case BC_ALLOC_OBJ_4:
            tmp_v.obj = alloc_obj(em, 5);
            READ_U32(tmp_int);
            tmp_v.obj->obj_header = (uint32_t)tmp_int;
            tmp_byte = OBJ_ENTRY_NUM(tmp_v);
            for (int i = 0; i < tmp_byte; ++i)
            {
                tmp_v.obj[tmp_byte - i] = POP();
            }
            PUSH(tmp_v);
            break;
        case BC_ALLOC_OBJ_5:
            tmp_v.obj = alloc_obj(em, 6);
            READ_U32(tmp_int);
            tmp_v.obj->obj_header = (uint32_t)tmp_int;
            tmp_byte = OBJ_ENTRY_NUM(tmp_v);
            for (int i = 0; i < tmp_byte; ++i)
            {
                tmp_v.obj[tmp_byte - i] = POP();
            }
            PUSH(tmp_v);
            break;
        case BC_ALLOC_OBJ_6:
            tmp_v.obj = alloc_obj(em, 7);
            READ_U32(tmp_int);
            tmp_v.obj->obj_header = (uint32_t)tmp_int;
            tmp_byte = OBJ_ENTRY_NUM(tmp_v);
            for (int i = 0; i < tmp_byte; ++i)
            {
                tmp_v.obj[tmp_byte - i] = POP();
            }
            PUSH(tmp_v);
            break;
        case BC_ALLOC_OBJ_U8:
            tmp_v.obj = alloc_obj(em, (int)READ_U8() + 1);
            READ_U32(tmp_int);
            tmp_v.obj->obj_header = (uint32_t)tmp_int;
            tmp_byte = OBJ_ENTRY_NUM(tmp_v);
            for (int i = 0; i < tmp_byte; ++i)
            {
                tmp_v.obj[tmp_byte - i] = POP();
            }
            PUSH(tmp_v);
            break;
        case BC_Peek:
            *sp = sp[-1];
            ++sp;
            break;
        case BC_POP_1:
            --sp;
            break;
        case BC_POP_2:
            sp -= 2;
            break;
        case BC_POP_3:
            sp -= 3;
            break;
        case BC_POP_4:
            sp -= 4;
            break;
        case BC_POP_5:
            sp -= 5;
            break;
        case BC_POP_6:
            sp -= 6;
            break;
        case BC_POP_U8:
            sp -= READ_U8();
            break;
        case BC_POP_U32:
            READ_U32(tmp_int);
            sp -= tmp_int;
            break;
        case BC_OBJ_TAG:
            tmp_int = OBJ_TAG(POP());
            sp->obj_header = (uint32_t)tmp_int;
            ++sp;
            break;
        case BC_OBJ_FIELD_0:
            tmp_v = POP().obj[1];
            PUSH(tmp_v);
            break;
        case BC_OBJ_FIELD_1:
            tmp_v = POP().obj[2];
            PUSH(tmp_v);
            break;
        case BC_OBJ_FIELD_2:
            tmp_v = POP().obj[3];
            PUSH(tmp_v);
            break;
        case BC_OBJ_FIELD_3:
            tmp_v = POP().obj[4];
            PUSH(tmp_v);
            break;
        case BC_OBJ_FIELD_4:
            tmp_v = POP().obj[5];
            PUSH(tmp_v);
            break;
        case BC_OBJ_FIELD_5:
            tmp_v = POP().obj[6];
            PUSH(tmp_v);
            break;
        case BC_OBJ_FIELD_6:
            tmp_v = POP().obj[7];
            PUSH(tmp_v);
            break;
        case BC_OBJ_FIELD_U8:
            tmp_v = POP().obj[READ_U8() + 1];
            PUSH(tmp_v);
            break;
        case BC_OBJ_FIELD_U32:
            READ_U32(tmp_int);
            tmp_v = POP().obj[tmp_int + 1];
            PUSH(tmp_v);
            break;
        case BC_ALLOC_LOCAL_1:
            sp += 1;
            break;
        case BC_ALLOC_LOCAL_2:
            sp += 2;
            break;
        case BC_ALLOC_LOCAL_3:
            sp += 3;
            break;
        case BC_ALLOC_LOCAL_4:
            sp += 4;
            break;
        case BC_ALLOC_LOCAL_5:
            sp += 5;
            break;
        case BC_ALLOC_LOCAL_6:
            sp += 6;
            break;
        case BC_ALLOC_LOCAL_U8:
            sp += READ_U8();
            break;
        case BC_ALLOC_LOCAL_U32:
            READ_U32(tmp_int);
            sp += (uint32_t)tmp_int;
            break;
        case BC_O_ACTION_0:
            output_actions[0](sp - 1);
            break;
        case BC_O_ACTION_1:
            output_actions[1](sp - 1);
            break;
        case BC_O_ACTION_2:
            output_actions[2](sp - 1);
            break;
        case BC_O_ACTION_3:
            output_actions[3](sp - 1);
            break;
        case BC_O_ACTION_U8:
            output_actions[READ_U8()](sp - 1);
            break;
        case BC_DROP_LOCAL_OBJ_0:
            drop(*bp);
            break;
        case BC_DROP_LOCAL_OBJ_1:
            drop(bp[1]);
            break;
        case BC_DROP_LOCAL_OBJ_2:
            drop(bp[2]);
            break;
        case BC_DROP_LOCAL_OBJ_3:
            drop(bp[3]);
            break;
        case BC_DROP_LOCAL_OBJ_4:
            drop(bp[4]);
            break;
        case BC_DROP_LOCAL_OBJ_5:
            drop(bp[5]);
            break;
        case BC_DROP_LOCAL_OBJ_6:
            drop(bp[6]);
            break;
        case BC_DROP_LOCAL_OBJ_I8:
            drop(bp[READ_I8()]);
            break;
        case BC_DROP_LOCAL_OBJ_I32:
            READ_I32(tmp_int);
            drop(bp[tmp_int]);
            break;
        case BC_GET_LOCAL_REF_0:
            PUSH(*bp);
            OBJ_INC_RC(*bp);
            break;
        case BC_GET_LOCAL_REF_1:
            tmp_v = bp[1];
            PUSH(tmp_v);
            OBJ_INC_RC(tmp_v);
            break;
        case BC_GET_LOCAL_REF_2:
            tmp_v = bp[2];
            PUSH(tmp_v);
            OBJ_INC_RC(tmp_v);
            break;
        case BC_GET_LOCAL_REF_3:
            tmp_v = bp[3];
            PUSH(tmp_v);
            OBJ_INC_RC(tmp_v);
            break;
        case BC_GET_LOCAL_REF_4:
            tmp_v = bp[4];
            PUSH(tmp_v);
            OBJ_INC_RC(tmp_v);
            break;
        case BC_GET_LOCAL_REF_5:
            tmp_v = bp[5];
            PUSH(tmp_v);
            OBJ_INC_RC(tmp_v);
            break;
        case BC_GET_LOCAL_REF_6:
            tmp_v = bp[6];
            PUSH(tmp_v);
            OBJ_INC_RC(tmp_v);
            break;
        case BC_GET_LOCAL_REF_I8:
            tmp_v = bp[READ_I8()];
            PUSH(tmp_v);
            OBJ_INC_RC(tmp_v);
            break;
        case BC_GET_LOCAL_REF_I32:
            READ_I32(tmp_int);
            tmp_v = bp[tmp_int];
            PUSH(tmp_v);
            OBJ_INC_RC(tmp_v);
            break;
        case BC_SET_LOCAL_REF_0:
            drop(*bp);
            bp[0] = POP();
            break;
        case BC_SET_LOCAL_REF_1:
            drop(bp[1]);
            bp[1] = POP();
            break;
        case BC_SET_LOCAL_REF_2:
            drop(bp[2]);
            bp[2] = POP();
            break;
        case BC_SET_LOCAL_REF_3:
            drop(bp[3]);
            bp[3] = POP();
            break;
        case BC_SET_LOCAL_REF_4:
            drop(bp[4]);
            bp[4] = POP();
            break;
        case BC_SET_LOCAL_REF_5:
            drop(bp[5]);
            bp[5] = POP();
            break;
        case BC_SET_LOCAL_REF_6:
            drop(bp[6]);
            bp[6] = POP();
            break;
        case BC_SET_LOCAL_REF_I8:
            tmp_int = READ_I8();
            drop(bp[tmp_int]);
            bp[tmp_int] = POP();
            break;
        case BC_SET_LOCAL_REF_I32:
            READ_I32(tmp_int);
            drop(bp[tmp_int]);
            bp[tmp_int] = POP();
            break;
        case BC_OBJ_FIELD_REF_0:
            tmp_v = POP().obj[1];
            OBJ_INC_RC(tmp_v);
            PUSH(tmp_v);
            break;
        case BC_OBJ_FIELD_REF_1:
            tmp_v = POP().obj[2];
            OBJ_INC_RC(tmp_v);
            PUSH(tmp_v);
            break;
        case BC_OBJ_FIELD_REF_2:
            tmp_v = POP().obj[3];
            OBJ_INC_RC(tmp_v);
            PUSH(tmp_v);
            break;
        case BC_OBJ_FIELD_REF_3:
            tmp_v = POP().obj[4];
            OBJ_INC_RC(tmp_v);
            PUSH(tmp_v);
            break;
        case BC_OBJ_FIELD_REF_4:
            tmp_v = POP().obj[5];
            OBJ_INC_RC(tmp_v);
            PUSH(tmp_v);
            break;
        case BC_OBJ_FIELD_REF_5:
            tmp_v = POP().obj[6];
            OBJ_INC_RC(tmp_v);
            PUSH(tmp_v);
            break;
        case BC_OBJ_FIELD_REF_6:
            tmp_v = POP().obj[7];
            OBJ_INC_RC(tmp_v);
            PUSH(tmp_v);
            break;
        case BC_OBJ_FIELD_REF_U8:
            tmp_v = OBJ_ITH(POP(), READ_U8());
            OBJ_INC_RC(tmp_v);
            PUSH(tmp_v);
            break;
        case BC_OBJ_FIELD_REF_U32:
            READ_U32(tmp_int);
            tmp_v = OBJ_ITH(POP(), (uint32_t)tmp_int);
            OBJ_INC_RC(tmp_v);
            PUSH(tmp_v);
            break;
        case BC_END_UPD_OBJ_U8:
            tmp_byte = READ_U8();
            drop(node_v[tmp_byte]);
            node_v[tmp_byte] = POP();
            --sp;
            ip = sp->ip;
            break;
        case BC_END_UPD_OBJ_U32:
            READ_U32(tmp_int);
            drop(node_v[(uint32_t)tmp_int]);
            node_v[(uint32_t)tmp_int] = POP();
            --sp;
            ip = sp->ip;
            break;
        case BC_GET_NODE_REF_U8:
            tmp_v = node_v[READ_U8()];
            OBJ_INC_RC(tmp_v);
            PUSH(tmp_v);
            break;
        case BC_GET_NODE_REF_U32:
            READ_U32(tmp_int);
            tmp_v = node_v[(uint32_t)tmp_int];
            OBJ_INC_RC(tmp_v);
            PUSH(tmp_v);
            break;
        case BC_GET_DATA_REF_U8:
            tmp_v = data[READ_U8()];
            OBJ_INC_RC(tmp_v);
            PUSH(tmp_v);
            break;
        case BC_GET_DATA_REF_U32:
            READ_U32(tmp_int);
            tmp_v = data[(uint32_t)tmp_int];
            OBJ_INC_RC(tmp_v);
            PUSH(tmp_v);
            break;
        case BC_GET_LAST_REF_0:
            tmp_v = node_vlast[0];
            OBJ_INC_RC(tmp_v);
            PUSH(tmp_v);
            break;
        case BC_GET_LAST_REF_1:
            tmp_v = node_vlast[1];
            OBJ_INC_RC(tmp_v);
            PUSH(tmp_v);
            break;
        case BC_GET_LAST_REF_2:
            tmp_v = node_vlast[2];
            OBJ_INC_RC(tmp_v);
            PUSH(tmp_v);
            break;
        case BC_GET_LAST_REF_3:
            tmp_v = node_vlast[3];
            OBJ_INC_RC(tmp_v);
            PUSH(tmp_v);
            break;
        case BC_GET_LAST_REF_U8:
            tmp_v = node_vlast[READ_U8()];
            OBJ_INC_RC(tmp_v);
            PUSH(tmp_v);
            break;
        case BC_GET_LAST_REF_U32:
            READ_U32(tmp_int);
            tmp_v = node_vlast[(uint32_t)tmp_int];
            OBJ_INC_RC(tmp_v);
            PUSH(tmp_v);
            break;
        case BC_SET_DATA_REF_U8:
            tmp_byte = READ_U8();
            drop(data[tmp_byte]);
            data[tmp_byte] = POP();
            break;
        case BC_SET_DATA_REF_U32:
            READ_U32(tmp_int);
            drop(data[tmp_int]);
            data[tmp_int] = POP();
            break;
        case BC_SET_LAST_REF_0:
            drop(node_vlast[0]);
            node_vlast[0] = POP();
            break;
        case BC_SET_LAST_REF_1:
            drop(node_vlast[1]);
            node_vlast[1] = POP();
            break;
        case BC_SET_LAST_REF_2:
            drop(node_vlast[2]);
            node_vlast[2] = POP();
            break;
        case BC_SET_LAST_REF_3:
            drop(node_vlast[3]);
            node_vlast[3] = POP();
            break;
        case BC_SET_LAST_REF_U8:
            tmp_byte = READ_U8();
            drop(node_vlast[tmp_byte]);
            node_vlast[tmp_byte] = POP();
            break;
        case BC_SET_LAST_REF_U32:
            READ_U32(tmp_int);
            drop(node_vlast[tmp_int]);
            node_vlast[tmp_int] = POP();
            break;
        case BC_SET_NODE_REF_U8:
            tmp_byte = READ_U8();
            drop(node_v[tmp_byte]);
            node_v[tmp_byte] = POP();
            break;
        case BC_SET_NODE_REF_U32:
            READ_U32(tmp_int);
            drop(node_v[tmp_int]);
            node_v[tmp_int] = POP();
            break;
        case BC_DROP_LAST_U8:
            drop(node_vlast[READ_U8()]);
            break;
        case BC_DROP_LAST_U32:
            READ_U32(tmp_int);
            drop(node_vlast[tmp_int]);
            break;
        case BC_Halt:
#ifdef EMFRP_DEBUG
            if (sp != em->v_stack)
            {
                em->res = EMFRP_PANIC;
            }
#endif
            return ZERO;
        case BC_ABORT:
            em->res = EMFRP_RUNTIME_ERR;
            return ZERO;

        case BC_Exit:
#ifdef EMFRP_DEBUG
            if ((em->v_stack + 1) != sp)
            {
                em->res = EMFRP_PANIC;
                return ZERO;
            }
#endif
            return *em->v_stack;
        default:
#ifdef EMFRP_DEBUG
            ////ESP_LOGI("error", "todo : %d\n", *ip);
            ESP_LOGI("error", "todo : %d\n", (int)ip[-1]);
            em->res = EMFRP_TODO;
#endif

            return ZERO;
        }
#ifdef EMFRP_DEBUG
        if ((sp - em->v_stack) < 0 || 128 <= (sp - em->v_stack))
        {
            ESP_LOGI("error", "stack overflow\n");
            em->res = EMFRP_PANIC;
            return ZERO;
        }
#endif
    }
}

void emfrp_init_vars(emfrp_machine *em, int n_node, int n_func, uint8_t **data)
{

    for (int i = 0; i < n_node; ++i)
    {
        int offset = next_int(data);
        int body_len = next_int(data);
        if (offset < em->node_list.len)
        {
            uint8_t *prev_body = em->node_list.action[offset].insns;
            free(prev_body);
        }
        uint8_t *body = (uint8_t *)malloc(body_len);
        memcpy(body, *data, body_len);
        *data += body_len;
#ifdef EMFRP_MEASURE_HEAP
        update_min_free_heap_size("init node");
#endif
        if (offset < em->node_list.len)
        {
            em->node_list.action[offset].insns = body;
        }
        else
        {
            OBJ_INC_RC(OBJ_INIT_VALUE);
            push_node_usr_uncheck(&em->node_list, OBJ_INIT_VALUE, body);
        }
    }
    for (int i = 0; i < n_func; ++i)
    {
        int offset = next_int(data);
        int body_len = next_int(data);
        if (offset < em->func_list.len)
        {
            free(em->func_list.lst[offset]);
        }
        uint8_t *body = (uint8_t *)malloc(body_len);
#ifdef EMFRP_MEASURE_HEAP
        update_min_free_heap_size("init func");

#endif
        memcpy(body, *data, body_len);
        *data += body_len;
        if (offset < em->func_list.len)
        {
            em->func_list.lst[offset] = body;
        }
        else
        {
            push_func_uncheck(&em->func_list, body);
        }
    }
}

value_t emfrp_new_bytecode(emfrp_machine *em, int data_len, uint8_t *data)
{
    int exp_len = next_int(&data);
    int upd_len = next_int(&data);
    int num_last = next_int(&data);
    int n_node = next_int(&data);
    int n_func = next_int(&data);
    int tmp;
    if (em->node_last != NULL)
        free(em->node_last);
    em->node_last = (value_t *)malloc(num_last * sizeof(value_t));

    tmp = next_int(&data);
    if (tmp > 0)
    {
        extend_node_list(&em->node_list, tmp + em->node_list.len);
    }
    tmp = next_int(&data);
    if (tmp > 0)
    {
        extend_func_list(&em->func_list, tmp + em->func_list.len);
    }
    tmp = next_int(&data);

    if (tmp > 0)
    {
        extend_data_list(&em->data_list, tmp + em->data_list.len);
        for (int i = 0; i < tmp; ++i)
        {
            OBJ_INC_RC(OBJ_INIT_VALUE);
            push_data_uncheck(&em->data_list, OBJ_INIT_VALUE);
        }
    }
    emfrp_init_vars(em, n_node, n_func, &data);
    if (upd_len > 0)
    {
        if (em->update != NULL)
            free(em->update);
        uint8_t *update = (uint8_t *)malloc(upd_len);
#ifdef EMFRP_MEASURE_HEAP
        update_min_free_heap_size("init copy update");
#endif
        memcpy(update, data, upd_len);
        data += upd_len;
        em->update = update;
    }
    if (exp_len == 0)
    {
        return ZERO;
    }
    else
    {
        em->res = EMFRP_OK;
        value_t tmp = emfrp_exec(em, data);
        return tmp;
    }
}

void emfrp_update(emfrp_machine *em)
{

    if (em->update == NULL)
    {
        em->res = EMFRP_NOTSTARTED;
        return;
    }
    else
    {

        em->res = EMFRP_OK;
        emfrp_exec(em, em->update);
#ifdef EMFRP_DEBUG
        ESP_LOGI("OBJ_CNT", "%d", obj_cnt);
#endif
        return;
    }
}
